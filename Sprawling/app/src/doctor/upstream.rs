// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::Mutex;

use kernel::Proxying;
use wire::{DoctorNewest, DoctorUnread, DoctorUpstream};

use super::REQUIREMENTS;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Upstream {

    Crate(&'static str),

    GitHub(&'static str),

    RustChannel,

    RustupChannel,

    PythonOrg,

    Unread(DoctorUnread),
}

const PATIENCE: std::time::Duration = std::time::Duration::from_secs(10);

const RUST_CHANNEL: &str = "https://static.rust-lang.org/dist/channel-rust-stable.toml";
const RUSTUP_CHANNEL: &str = "https://static.rust-lang.org/rustup/release-stable.toml";
const PYTHON_RELEASES: &str =
    "https://www.python.org/api/v2/downloads/release/?is_published=true&pre_release=false";

static KNOWN: Mutex<BTreeMap<String, DoctorNewest>> = Mutex::new(BTreeMap::new());

pub(crate) fn newest(item: &str) -> DoctorUpstream {
    let answer = |newest| DoctorUpstream {
        item: item.to_owned(),
        newest,
    };
    let Some(row) = REQUIREMENTS.iter().find(|row| row.name == item) else {
        return answer(DoctorNewest::Unread {
            why: DoctorUnread::UnknownItem,
        });
    };
    if let Upstream::Unread(why) = row.upstream {
        return answer(DoctorNewest::Unread { why });
    }
    let Ok(mut known) = KNOWN.lock() else {
        return answer(DoctorNewest::Refused {
            said: "a reading of this item stopped part way; restart this city to ask again"
                .to_owned(),
        });
    };
    match known.get(item).cloned() {
        Some(DoctorNewest::Refused { said }) => {
            known.remove(item);
            answer(DoctorNewest::Refused { said })
        }
        Some(held) => answer(held),
        None => {
            known.insert(item.to_owned(), DoctorNewest::Asking);
            drop(known);
            answer(start_reading(row.name, row.upstream))
        }
    }
}

fn start_reading(item: &'static str, upstream: Upstream) -> DoctorNewest {
    let started = std::thread::Builder::new()
        .name(format!("upstream-{item}"))
        .spawn(move || {
            let reading = read(upstream);
            if let Ok(mut known) = KNOWN.lock() {
                known.insert(item.to_owned(), reading);
            }
        });
    match started {
        Ok(_reading) => DoctorNewest::Asking,
        Err(err) => {
            if let Ok(mut known) = KNOWN.lock() {
                known.remove(item);
            }
            DoctorNewest::Refused {
                said: format!("no thread to ask with: {err}"),
            }
        }
    }
}

fn read(upstream: Upstream) -> DoctorNewest {
    let reading = match upstream {
        Upstream::Unread(why) => return DoctorNewest::Unread { why },
        Upstream::Crate(name) => fetch(&format!("https://crates.io/api/v1/crates/{name}"))
            .and_then(|body| of_crate(&body)),
        Upstream::GitHub(repo) => fetch(&format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .and_then(|body| of_github(&body)),
        Upstream::RustChannel => fetch(RUST_CHANNEL).and_then(|body| of_rust_channel(&body)),
        Upstream::RustupChannel => fetch(RUSTUP_CHANNEL).and_then(|body| of_rustup_channel(&body)),
        Upstream::PythonOrg => fetch(PYTHON_RELEASES).and_then(|body| of_python_org(&body)),
    };
    match reading {
        Ok(version) => DoctorNewest::Read { version },
        Err(said) => DoctorNewest::Refused { said },
    }
}

fn fetch(url: &str) -> Result<String, String> {
    let client = gateway::client_for(Proxying::ExceptLocal, url)
        .timeout(PATIENCE)
        .build()
        .map_err(|err| format!("no HTTP client: {err}"))?;
    let response = client
        .get(url)
        .header("accept", "application/json, text/plain")
        .send()
        .map_err(|err| format!("{}: {err}", host_of(url)))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{} answered {status}", host_of(url)));
    }
    response
        .text()
        .map_err(|err| format!("{}: {err}", host_of(url)))
}

fn host_of(url: &str) -> &str {
    url.split('/').nth(2).unwrap_or(url)
}

pub(crate) fn dotted(text: &str) -> Option<String> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let rest = text.get(start..)?;
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let number = rest.get(..end)?.trim_end_matches('.');
    number.contains('.').then(|| number.to_owned())
}

fn json(body: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(body).map_err(|err| format!("the answer was not JSON: {err}"))
}

fn of_crate(body: &str) -> Result<String, String> {
    json(body)?
        .pointer("/crate/max_stable_version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "crates.io named no stable version".to_owned())
}

fn of_github(body: &str) -> Result<String, String> {
    json(body)?
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .and_then(dotted)
        .ok_or_else(|| "the latest release carried no version".to_owned())
}

fn of_rust_channel(body: &str) -> Result<String, String> {
    body.split_once("\n[pkg.rust]\n")
        .and_then(|(_, rest)| rest.lines().find(|line| line.starts_with("version")))
        .and_then(dotted)
        .ok_or_else(|| "the channel manifest named no rust version".to_owned())
}

fn of_rustup_channel(body: &str) -> Result<String, String> {
    body.lines()
        .find(|line| line.starts_with("version"))
        .and_then(dotted)
        .ok_or_else(|| "the rustup channel named no version".to_owned())
}

fn of_python_org(body: &str) -> Result<String, String> {
    let releases = json(body)?;
    releases
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|release| release.get("name").and_then(serde_json::Value::as_str))
        .filter_map(|name| name.strip_prefix("Python "))
        .filter_map(|version| {
            let parts: Option<Vec<u32>> =
                version.split('.').map(|part| part.parse().ok()).collect();
            parts.map(|parts| (parts, version.to_owned()))
        })
        .max()
        .map(|(_, version)| version)
        .ok_or_else(|| "python.org listed no final release".to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn each_source_is_read_as_the_version_it_names() {
        let rust = "manifest-version = \"2\"\n[pkg.cargo]\nversion = \"0.99.0 (797e8a9bc 2026-08-05)\"\n\
                    [pkg.rust]\nversion = \"1.98.0 (1a2b3c4d5 2026-08-06)\"\n";
        let python = r#"[{"name":"Python 3.9.1"},{"name":"Python 3.14.2"},{"name":"Python 3.14.10"},{"name":"Python 2.7.18"}]"#;
        assert_eq!(
            (
                of_crate(
                    r#"{"crate":{"max_stable_version":"0.9.143","max_version":"0.10.0-rc.1"}}"#
                ),
                of_github(r#"{"tag_name":"bun-v1.4.2"}"#),
                of_github(r#"{"tag_name":"v2.55.0.windows.5"}"#),
                of_rust_channel(rust),
                of_rustup_channel("schema-version = '1'\nversion = '1.29.1'\n"),
                of_python_org(python),
            ),
            (
                Ok("0.9.143".to_owned()),
                Ok("1.4.2".to_owned()),
                Ok("2.55.0".to_owned()),
                Ok("1.98.0".to_owned()),
                Ok("1.29.1".to_owned()),
                Ok("3.14.10".to_owned()),
            )
        );
    }

    #[test]
    fn an_answer_in_another_shape_is_refused() {
        assert_eq!(
            (
                of_crate("<html>login</html>").is_err(),
                of_github(r#"{"message":"Not Found"}"#).is_err(),
                of_rust_channel("[pkg.cargo]\nversion = \"0.99.0\"\n").is_err(),
            ),
            (true, true, true)
        );
    }

    #[test]
    fn an_item_with_no_source_is_answered_without_asking_anyone() {
        assert_eq!(
            (newest("no-such-item").newest, newest("rustfmt").newest),
            (
                DoctorNewest::Unread {
                    why: DoctorUnread::UnknownItem
                },
                DoctorNewest::Unread {
                    why: DoctorUnread::WithToolchain
                }
            )
        );
    }
}
