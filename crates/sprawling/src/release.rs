// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which release this binary is, and which release the registry offers.
//!
//! **Nothing here runs unless a person asks.** No timer, no first-run
//! probe, no check folded into another command: `status` reads what is
//! compiled in and touches no socket, and only `status --check` calls
//! out. `QUICKSTART.md` opens by promising that nothing was installed
//! and nothing outside the folder was written, and a binary that phoned
//! a registry on its own schedule would be breaking that sentence for a
//! question nobody asked.
//!
//! **Nothing here updates anything either.** Where a binary lives
//! belongs to whoever installed it - `sprawling install` owns the
//! archive path, npm and bun own theirs (`tools/xtask/src/channel/shim.js`) - so this
//! reports and stops. The answer names the command to run, which keeps
//! the person who chose an install channel in charge of it.
//!
//! **The registry is the source, not GitHub.** Every release of this
//! project is a pre-release, and `GET /repos/{owner}/{repo}/releases/latest`
//! excludes pre-releases by design: it answers 404 for this repository
//! today and would answer with a stale release the day one is promoted.
//! npm's `latest` dist-tag is what `bunx sprawling` resolves, so asking
//! it is asking the question a person actually has.

use kernel::release::Version;
use kernel::{AxCode, AxError, Proxying, Reach, Release};
use wire::{InstallChannel, Registry, RegistryNewest, RegistryReading};
use wire::{ReleaseAnswer, ReleaseLine, UpdateHint};

/// The root package `release.yml` publishes, and the name `bunx
/// sprawling` resolves. Its `latest` dist-tag is the newest release by
/// definition, because the publish step passes `--tag latest` for every
/// release this project makes.
const LATEST_URL: &str = "https://registry.npmjs.org/sprawling/latest";

/// The crate `release.yml` publishes to crates.io, the one `cargo
/// install sprawling` resolves.
const CRATES_URL: &str = "https://crates.io/api/v1/crates/sprawling";

/// The client both registries are asked as. crates.io refuses a request
/// whose User-Agent does not name the program making it, and npm reads
/// the same header without requiring it.
const USER_AGENT: &str = concat!(
    "sprawling/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/2youg1/sprawling-agents)"
);

/// Where every archive is, whether or not npm could be reached. The one
/// answer that is useful when this command cannot give its own.
const RELEASES: &str = "https://github.com/2youg1/sprawling-agents/releases";

/// The command that updates a binary npm or bun installed, printed and
/// never run (`crates/wire/spec/Answer/Release.lean` D24).
const NPM_UPDATE: &str = "npm install -g sprawling@latest";

/// The command that updates a binary cargo installed. `cargo binstall`
/// puts its binary in the same directory and is answered the same way,
/// because the path cannot tell the two apart and this command updates
/// either.
const CARGO_UPDATE: &str = "cargo install sprawling --locked";

/// A directory a release archive carries beside the binary, and no
/// other install channel does (`tools/xtask/src/package/contents.rs`).
const ARCHIVE_SIBLING: &str = "skills";

/// One version manifest is a few hundred bytes, so a reader waiting on
/// it has either been answered or is not going to be. Long enough for a
/// slow link, short enough that a blocked network is a pause rather than
/// something a person kills.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(10);

/// Whether this binary came out of a release, and which one.
///
/// Two states rather than an `Option<Release>`, because the absent case
/// is a fact about the build rather than a missing value: a binary built
/// from a working tree has no release to compare, and reporting it as
/// out of date would be answering a question about a different binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Built {
    /// Built by the release workflow, from this tag.
    Released(Release),
    /// Built from a working tree. `cargo build` sets no tag, and this is
    /// what that absence means.
    FromSource,
}

/// What this binary was built as.
///
/// The tag arrives through `SPRAWLING_RELEASE_TAG`, which only
/// `release.yml` sets; `build.rs` declares the variable so cargo rebuilds
/// when it changes rather than serving a cached binary that names the
/// previous release.
///
/// # Errors
/// When a tag was set and does not name a release of this version. That
/// is a mislabelled build, and it fails here rather than telling every
/// reader the wrong thing about what they are running.
pub fn built() -> Result<Built, AxError> {
    match option_env!("SPRAWLING_RELEASE_TAG") {
        None => Ok(Built::FromSource),
        // An empty value is what a workflow sets when the variable is
        // referenced in a context that has no tag, and it means the same
        // as never having set it.
        Some("") => Ok(Built::FromSource),
        Some(tag) => Release::from_tag(tag, env!("CARGO_PKG_VERSION")).map(Built::Released),
    }
}

/// The newest release npm offers.
///
/// # Errors
/// When the registry cannot be reached, answers something other than a
/// version manifest, or names a version this project did not publish.
/// The recovery carries the staged reading, so a person behind a proxy
/// is told where the call stopped rather than that it "failed".
pub fn newest() -> Result<Release, AxError> {
    npm_newest(LATEST_URL)
}

/// npm's `latest` manifest at `url`, read as a release.
fn npm_newest(url: &str) -> Result<Release, AxError> {
    Release::from_npm_version(&manifest_field(url, &["version"])?)
}

/// The newest version crates.io offers at `url`. `max_version` is the
/// highest version the crate carries, and every version this project
/// uploads is a bare `x.y.z`, so it is the newest release
/// (`crates/kernel/spec/Release.lean` §8-54-1).
fn crates_newest(url: &str) -> Result<Version, AxError> {
    Version::from_crates_version(&manifest_field(url, &["crate", "max_version"])?)
}

/// One string out of the JSON document at `url`, found by walking
/// `path`. One field rather than a shape that would have to be revised
/// whenever a registry adds another: this asks which version is newest
/// and nothing else.
fn manifest_field(url: &str, path: &[&str]) -> Result<String, AxError> {
    let client = gateway::client_for(Proxying::ExceptLocal, url)
        .timeout(PATIENCE)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ToolUnavailable, "ask the registry", err.to_string())
                .with_recovery("this machine refused to build an HTTP client")
        })?;
    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .map_err(|_| stopped(&client, url))?;
    let status = response.status();
    if !status.is_success() {
        return Err(
            AxError::failure(AxCode::ToolUnavailable, "ask the registry", url.to_owned())
                .with_recovery(format!(
                    "the registry answered {status}; the release page carries every \
             archive either way: {RELEASES}"
                )),
        );
    }
    let body: serde_json::Value = response.json().map_err(|err| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "read the registry's answer",
            err.to_string(),
        )
        .with_recovery(
            "the registry answered something other than a version manifest; \
             a proxy that returns a login page does this",
        )
    })?;
    path.iter()
        .try_fold(&body, |at, key| at.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "read the registry's answer",
                url.to_owned(),
            )
            .with_recovery(format!("the manifest carried no `{}`", path.join(".")))
        })
}

/// Where the call stopped, as the vocabulary the endpoint page already
/// uses. Costs a second request and only on the failure path, which is
/// the path where "it did not work" is not an answer a person can act
/// on.
fn stopped(client: &reqwest::blocking::Client, url: &str) -> AxError {
    let Reach {
        host,
        named,
        connected,
        answered,
        through,
        ..
    } = gateway::reach(client, Proxying::ExceptLocal, url, 0);
    AxError::failure(AxCode::ToolUnavailable, "ask the registry", host).with_recovery(format!(
        "the call stopped there: name {named:?}, socket {connected:?}, \
         request {answered:?}, through {through:?}"
    ))
}

/// One release, as the two strings a page and a terminal both print.
fn line(release: &Release) -> ReleaseLine {
    ReleaseLine {
        version: release.npm_version(),
        released: release.released(),
    }
}

/// crates.io's version as a line. `released` is empty because the
/// registry's version carries no date, and the city does not make one up
/// (`crates/wire/spec/Answer/Release.lean`).
fn crates_line(version: &Version) -> ReleaseLine {
    ReleaseLine {
        version: version.to_string(),
        released: String::new(),
    }
}

/// Where each registry is asked. Production asks the two public ones; a
/// test points both at a loopback stand-in.
struct Registries<'a> {
    npm: &'a str,
    crates: &'a str,
}

/// Both readings, taken and judged.
///
/// **One authority for the whole check.** `status --check` and
/// [`Query::NewestRelease`](wire::Query::NewestRelease) render the same value,
/// so the terminal and the page cannot come to different conclusions
/// about one binary.
///
/// Never fails: a check that could not be made is an answer a person
/// has to see, and returning it as an error would let a caller drop it
/// and draw nothing. What this binary is, is read first, so a
/// mislabelled build is reported without a request nobody can use.
#[must_use]
pub fn answer() -> ReleaseAnswer {
    match built() {
        Ok(mine) => judged(
            mine,
            this_channel(),
            &Registries {
                npm: LATEST_URL,
                crates: CRATES_URL,
            },
        ),
        Err(refusal) => ReleaseAnswer::Refused { refusal },
    }
}

/// Asks both registries and judges `mine` against the one its install
/// channel updates from (`crates/wire/spec/Answer/Release.lean`): a binary
/// cargo installed against crates.io, every other one against npm. When
/// that registry cannot be read the whole answer is that refusal.
fn judged(mine: Built, installed: InstallChannel, at: &Registries<'_>) -> ReleaseAnswer {
    let npm = npm_newest(at.npm);
    let crates = crates_newest(at.crates);
    let registries = vec![
        RegistryNewest {
            registry: Registry::Npm,
            reading: reading(npm.as_ref().map(line)),
        },
        RegistryNewest {
            registry: Registry::CratesIo,
            reading: reading(crates.as_ref().map(crates_line)),
        },
    ];
    let mine = match mine {
        Built::FromSource => {
            return ReleaseAnswer::Unreleased {
                registries,
                update: hint(InstallChannel::Source),
            };
        }
        Built::Released(mine) => mine,
    };
    let verdict = match installed {
        InstallChannel::Cargo => {
            crates.map(|newest| kernel::release::stands_on_crates(&mine, &newest))
        }
        InstallChannel::Npm | InstallChannel::Archive | InstallChannel::Source => {
            npm.map(|newest| kernel::release::stands(&mine, &newest))
        }
    };
    match verdict {
        Ok(verdict) => ReleaseAnswer::Stands {
            verdict,
            mine: line(&mine),
            registries,
            update: hint(installed),
        },
        Err(refusal) => ReleaseAnswer::Refused { refusal },
    }
}

/// One registry's answer as the page shows it.
fn reading(read: Result<ReleaseLine, &AxError>) -> RegistryReading {
    match read {
        Ok(newest) => RegistryReading::Read { newest },
        Err(refusal) => RegistryReading::Refused {
            refusal: refusal.clone(),
        },
    }
}

/// How the running binary was installed, read from its own path.
///
/// A binary that cannot name its own path cannot say how it got there,
/// so it is answered as one built from source: no command is printed
/// rather than one for a channel this binary did not come through.
fn this_channel() -> InstallChannel {
    // cargo's own rule: `CARGO_HOME` when set, else `.cargo` under the
    // home directory, whose reading `accounting::home` owns.
    let cargo_home = match std::env::var_os("CARGO_HOME") {
        Some(set) => Some(std::path::PathBuf::from(set)),
        None => match accounting::home::Home::detect() {
            Ok(home) => Some(home.path().join(".cargo")),
            Err(_no_home) => None,
        },
    };
    let cargo_bin = cargo_home.map(|cargo_home| cargo_home.join("bin"));
    match std::env::current_exe() {
        Ok(exe) => channel(&exe, cargo_bin.as_deref()),
        Err(_unnamed) => InstallChannel::Source,
    }
}

/// The channel a binary at `exe` came through, on every platform by the
/// same rule (`crates/wire/spec/Answer/Release.lean` D24): under a
/// `node_modules`, bun's or npx's package cache is npm; in cargo's bin
/// directory is cargo; beside the archive's `skills/` is the archive;
/// anywhere else is a build nobody published.
pub(crate) fn channel(
    exe: &std::path::Path,
    cargo_bin: Option<&std::path::Path>,
) -> InstallChannel {
    let packaged = exe.components().any(|part| {
        matches!(
            part.as_os_str().to_str(),
            Some("node_modules" | ".bun" | "_npx")
        )
    });
    let dir = exe.parent();
    if packaged {
        InstallChannel::Npm
    } else if cargo_bin.is_some() && dir == cargo_bin {
        InstallChannel::Cargo
    } else if dir.is_some_and(|dir| dir.join(ARCHIVE_SIBLING).is_dir()) {
        InstallChannel::Archive
    } else {
        InstallChannel::Source
    }
}

/// The command a User runs to update a binary from `channel`.
fn hint(channel: InstallChannel) -> UpdateHint {
    let command = match channel {
        InstallChannel::Npm => Some(NPM_UPDATE.to_owned()),
        InstallChannel::Cargo => Some(CARGO_UPDATE.to_owned()),
        InstallChannel::Archive => Some(format!(
            "download the newest archive from {RELEASES}, then run `sprawling install` from it"
        )),
        InstallChannel::Source => None,
    };
    UpdateHint { channel, command }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests {
    use super::{Built, Registries, built, channel, judged};
    use kernel::ReleaseVerdict;
    use std::io::{BufRead, BufReader, Write};
    use wire::{InstallChannel, ReleaseAnswer};

    /// A loopback stand-in for both registries: `/npm` answers npm's
    /// manifest, `/crates` crates.io's, every request closes its
    /// connection, and the User-Agent of each request is sent back to the
    /// test.
    fn registries(
        npm: &'static str,
        crates: &'static str,
    ) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (agents, seen) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut head = Vec::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    head.push(line);
                }
                let body = if head[0].contains("/crates") {
                    crates
                } else {
                    npm
                };
                let agent = head
                    .iter()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("user-agent:")
                            .map(|it| it.trim().to_owned())
                    })
                    .unwrap_or_default();
                agents.send(agent).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (base, seen)
    }

    /// npm carries a re-cut of 0.0.8 with a later date, which crates.io
    /// cannot carry: a binary cargo installed is current against the
    /// crates.io answer of its own release, and one npm installed is told
    /// it is behind.
    #[test]
    fn a_crates_answer_of_the_same_release_reads_as_current() {
        let (base, seen) = registries(
            r#"{"version":"0.0.8-pre.261005"}"#,
            r#"{"crate":{"max_version":"0.0.8"}}"#,
        );
        let npm = format!("{base}/npm");
        let crates = format!("{base}/crates");
        let at = Registries {
            npm: &npm,
            crates: &crates,
        };
        let mine =
            || Built::Released(kernel::Release::from_npm_version("0.0.8-pre.261002").unwrap());
        let verdict = |answer: ReleaseAnswer| match answer {
            ReleaseAnswer::Stands { verdict, .. } => verdict,
            other => panic!("not judged: {other:?}"),
        };
        assert_eq!(
            [
                verdict(judged(mine(), InstallChannel::Cargo, &at)),
                verdict(judged(mine(), InstallChannel::Npm, &at)),
            ],
            [ReleaseVerdict::Current, ReleaseVerdict::Behind]
        );
        let agents: Vec<String> = seen.try_iter().collect();
        assert_eq!(agents.len(), 4);
        assert!(
            agents.iter().all(|agent| agent.starts_with("sprawling/")),
            "{agents:?}"
        );
    }

    /// The four channels, read from fixture paths rather than from where
    /// this test binary happens to run. Each OS spells its paths its own
    /// way; the rule reads components, so one fixture set covers all three.
    #[test]
    fn the_update_command_follows_the_channel_the_binary_came_through() {
        let root = tempfile::tempdir().unwrap();
        let cargo_bin = root.path().join(".cargo").join("bin");
        let archive = root.path().join("sprawling-0.0.9");
        std::fs::create_dir_all(archive.join("skills")).unwrap();
        let exe = |dir: &std::path::Path| dir.join("sprawling");
        let npm = root
            .path()
            .join("lib")
            .join("node_modules")
            .join("sprawling")
            .join("bin");
        let bunx = root.path().join(".bun").join("install").join("cache");
        assert_eq!(
            [
                channel(&exe(&npm), Some(&cargo_bin)),
                channel(&exe(&bunx), Some(&cargo_bin)),
                channel(&exe(&cargo_bin), Some(&cargo_bin)),
                channel(&exe(&archive), Some(&cargo_bin)),
                channel(&exe(&root.path().join("target")), Some(&cargo_bin)),
                channel(&exe(&cargo_bin), None),
            ],
            [
                InstallChannel::Npm,
                InstallChannel::Npm,
                InstallChannel::Cargo,
                InstallChannel::Archive,
                InstallChannel::Source,
                InstallChannel::Source,
            ]
        );
    }

    /// The check this test exists for is not which state a test binary
    /// is in, but that reading it never panics and never invents a
    /// release: `cargo test` sets no tag, and a build that did set one
    /// must decode it rather than carry it as text.
    #[test]
    fn a_binary_knows_whether_it_came_out_of_a_release() {
        match built().expect("this build's own tag decodes") {
            Built::FromSource => {}
            Built::Released(mine) => {
                assert_eq!(mine.version(), env!("CARGO_PKG_VERSION"));
                assert_eq!(
                    kernel::Release::from_npm_version(&mine.npm_version())
                        .expect("what this release publishes"),
                    mine
                );
            }
        }
    }
}
