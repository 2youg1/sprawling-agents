// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The newest release of one item, asked of its publisher
//! (sprawling-SPEC.md section 8-120).
//!
//! **Only official sources**: crates.io for a crate, the Rust and rustup
//! release channels for those two, python.org for Python, and a
//! project's own GitHub releases for the rest. Every call leaves through
//! `gateway::client_for`, the proxy rule every other outbound call of
//! this city follows.
//!
//! **A reading that arrived is kept for the life of this process.**
//! GitHub gives a caller without credentials sixty questions an hour,
//! and a page that asked again on every opening would spend them in an
//! afternoon. A refused reading is not kept, so the next opening asks
//! again. Nothing expires by time, because this binary reads the clock
//! in `bin::assembly` alone; a restarted city asks afresh.

use std::collections::BTreeMap;
use std::sync::Mutex;

use channels::{DoctorNewest, DoctorUnread, DoctorUpstream};
use kernel::Proxying;

use super::REQUIREMENTS;

/// Where an item's newest release is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Upstream {
    /// A crate on crates.io, by its crate name.
    Crate(&'static str),
    /// A GitHub repository's latest release, `owner/name`.
    GitHub(&'static str),
    /// The stable Rust release channel's manifest.
    RustChannel,
    /// rustup's own release channel.
    RustupChannel,
    /// python.org's list of published releases.
    PythonOrg,
    /// Nothing to ask, and the reason a page words.
    Unread(DoctorUnread),
}

/// One manifest is at most a megabyte, so a reader waiting on it has
/// been answered or is not going to be; the same patience the release
/// check gives npm.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(10);

const RUST_CHANNEL: &str = "https://static.rust-lang.org/dist/channel-rust-stable.toml";
const RUSTUP_CHANNEL: &str = "https://static.rust-lang.org/rustup/release-stable.toml";
const PYTHON_RELEASES: &str =
    "https://www.python.org/api/v2/downloads/release/?is_published=true&pre_release=false";

/// The readings already taken in this process, by item.
static KNOWN: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

/// The newest release of `item`, or why there is none to show. Never
/// fails: a source that did not answer is an answer a page draws.
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
    if let Some(version) = KNOWN.lock().ok().and_then(|known| known.get(item).cloned()) {
        return answer(DoctorNewest::Read { version });
    }
    let newest = read(row.upstream);
    if let DoctorNewest::Read { version } = &newest
        && let Ok(mut known) = KNOWN.lock()
    {
        known.insert(item.to_owned(), version.clone());
    }
    answer(newest)
}

/// Asks the source `upstream` names.
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

/// The body `url` answers with, or where the call stopped.
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

/// The first dotted number in `text`: `bun-v1.4.2` and
/// `v2.55.0.windows.5` both read as the version a page compares.
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

/// The `version` under `[pkg.rust]`, which reads `1.97.1 (hash date)`.
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

/// The greatest `Python X.Y.Z` among the published final releases.
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

    /// One answer of each source's shape, cut down to what is read.
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

    /// A body in some other shape is a refusal that says so, never a
    /// version made up out of whatever number the body held.
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
    fn an_item_the_table_does_not_carry_is_named_as_unknown() {
        assert_eq!(
            newest("no-such-item").newest,
            DoctorNewest::Unread {
                why: DoctorUnread::UnknownItem
            }
        );
    }
}
