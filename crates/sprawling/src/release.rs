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

use kernel::{AxCode, AxError, Proxying, Reach, Release};
use wire::{InstallChannel, Registry, RegistryNewest, RegistryReading};
use wire::{ReleaseAnswer, ReleaseLine, UpdateHint};

/// The root package `release.yml` publishes, and the name `bunx
/// sprawling` resolves. Its `latest` dist-tag is the newest release by
/// definition, because the publish step passes `--tag latest` for every
/// release this project makes.
const LATEST_URL: &str = "https://registry.npmjs.org/sprawling/latest";

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

/// The newest release the registry offers.
///
/// # Errors
/// When the registry cannot be reached, answers something other than a
/// version manifest, or names a version this project did not publish.
/// The recovery carries the staged reading, so a person behind a proxy
/// is told where the call stopped rather than that it "failed".
pub fn newest() -> Result<Release, AxError> {
    let client = gateway::client_for(Proxying::ExceptLocal, LATEST_URL)
        .timeout(PATIENCE)
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ToolUnavailable, "ask the registry", err.to_string())
                .with_recovery("this machine refused to build an HTTP client")
        })?;
    let response = client
        .get(LATEST_URL)
        .header("accept", "application/json")
        .send()
        .map_err(|_| stopped(&client))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AxError::failure(
            AxCode::ToolUnavailable,
            "ask the registry",
            LATEST_URL.to_owned(),
        )
        .with_recovery(format!(
            "the registry answered {status}; the release page carries every \
             archive either way: {RELEASES}"
        )));
    }
    // One field out of the manifest, rather than a shape that would have
    // to be revised whenever npm adds another: this asks which version is
    // newest and nothing else.
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
    let version = body
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "read the registry's answer",
                LATEST_URL.to_owned(),
            )
            .with_recovery("the manifest carried no `version`")
        })?;
    Release::from_npm_version(version)
}

/// Where the call stopped, as the vocabulary the endpoint page already
/// uses. Costs a second request and only on the failure path, which is
/// the path where "it did not work" is not an answer a person can act
/// on.
fn stopped(client: &reqwest::blocking::Client) -> AxError {
    let Reach {
        host,
        named,
        connected,
        answered,
        through,
        ..
    } = gateway::reach(client, Proxying::ExceptLocal, LATEST_URL, 0);
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
    let mine = match built() {
        Ok(found) => found,
        Err(refusal) => return ReleaseAnswer::Refused { refusal },
    };
    let newest = match newest() {
        Ok(found) => found,
        Err(refusal) => return ReleaseAnswer::Refused { refusal },
    };
    let registries = vec![
        RegistryNewest {
            registry: Registry::Npm,
            reading: RegistryReading::Read {
                newest: line(&newest),
            },
        },
        RegistryNewest {
            registry: Registry::CratesIo,
            reading: RegistryReading::Unasked,
        },
    ];
    match mine {
        Built::FromSource => ReleaseAnswer::Unreleased {
            registries,
            update: UpdateHint {
                channel: InstallChannel::Source,
                command: None,
            },
        },
        Built::Released(mine) => ReleaseAnswer::Stands {
            verdict: kernel::release::stands(&mine, &newest),
            mine: line(&mine),
            registries,
            update: hint(this_channel()),
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
    reason = "test code"
)]
mod tests {
    use super::{Built, built, channel};
    use wire::InstallChannel;

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
