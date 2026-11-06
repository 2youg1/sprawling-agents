// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! Manual checks: install origin selects the registry, comparison operand and updater.
//! Specified by `crates/wire/spec/Answer/Release.lean` and `crates/sprawling/spec/Install.lean`.
use kernel::release::Version;
use kernel::{AxCode, AxError, Release};
use wire::{InstallChannel, Registry, RegistryNewest, RegistryReading};
use wire::{ReleaseAnswer, ReleaseLine, UpdateHint};
const LATEST_URL: &str = "https://registry.npmjs.org/sprawling/latest";
const CRATES_URL: &str = "https://crates.io/api/v1/crates/sprawling";
/// The marker the archive installer places beside its binary.
pub const ARCHIVE_ORIGIN_FILE: &str = ".sprawling-archive-origin";
const RELEASES: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/releases");
const NPM_UPDATE: &str = "npm install -g sprawling@latest";
const BUN_UPDATE: &str = "bun install -g sprawling@latest";
const BINSTALL_UPDATE: &str = "cargo binstall sprawling";
const CARGO_UPDATE: &str = "cargo install sprawling --locked";
const AUR_UPDATE: &str = "git pull --ff-only && makepkg -si";
const ARCHIVE_SIBLING: &str = "skills";
/// Build identity; source builds carry no invented release tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Built {
    /// Built by the release workflow, from this tag.
    Released(Release),
    /// Built from a UTC-dated main snapshot.
    Nightly(NightlyBuild),
    /// Compiled without a release tag, including Cargo source installations.
    FromSource,
}

pub use nightly::NightlyBuild;

/// Reads the build tag.
/// # Errors
/// Refuses a tag that disagrees with the Cargo version or maturity.
pub fn built() -> Result<Built, AxError> {
    let curated = option_env!("SPRAWLING_RELEASE_TAG").filter(|tag| !tag.is_empty());
    let nightly = option_env!("SPRAWLING_NIGHTLY_TAG").filter(|tag| !tag.is_empty());
    let source = option_env!("SPRAWLING_SOURCE_SHA").filter(|sha| !sha.is_empty());
    match (curated, nightly, source) {
        (Some(tag), None, None) => {
            Release::from_tag(tag, env!("CARGO_PKG_VERSION")).map(Built::Released)
        }
        (None, Some(tag), Some(source)) => NightlyBuild::new(tag, source).map(Built::Nightly),
        (None, None, None) => Ok(Built::FromSource),
        _ => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read the build identity",
            "release environment",
        )
        .with_recovery(
            "set SPRAWLING_RELEASE_TAG alone, or set SPRAWLING_NIGHTLY_TAG and \
             SPRAWLING_SOURCE_SHA together; unset all three for a source build",
        )),
    }
}

fn line(release: &Release) -> ReleaseLine {
    ReleaseLine {
        version: release.npm_version(),
        released: release.released(),
    }
}

fn crates_line(version: &Version) -> ReleaseLine {
    ReleaseLine {
        version: version.to_string(),
        released: String::new(),
    }
}

struct Registries<'a> {
    npm: &'a str,
    crates: &'a str,
    github: &'a str,
    compare: &'a str,
}

enum GithubAnswer {
    Curated(registry::GithubRelease),
    Nightly(NightlyBuild),
}

impl GithubAnswer {
    fn line(&self) -> ReleaseLine {
        match self {
            GithubAnswer::Curated(release) => line(&release.cut),
            GithubAnswer::Nightly(build) => ReleaseLine {
                version: build.tag().to_owned(),
                released: String::new(),
            },
        }
    }
}

/// Reads registries only on request; returns the selected comparison or a refusal.
#[must_use]
pub fn answer() -> ReleaseAnswer {
    let repository = env!("CARGO_PKG_REPOSITORY")
        .trim_start_matches("https://github.com/")
        .trim_end_matches('/');
    let github = format!("https://api.github.com/repos/{repository}/releases");
    let compare = format!("https://api.github.com/repos/{repository}/compare");
    match built() {
        Ok(mine) => judged(
            mine,
            this_channel(),
            &Registries {
                npm: LATEST_URL,
                crates: CRATES_URL,
                github: &github,
                compare: &compare,
            },
        ),
        Err(refusal) => ReleaseAnswer::Refused { refusal },
    }
}

mod nightly;
mod registry;

fn judged(mine: Built, installed: InstallChannel, at: &Registries<'_>) -> ReleaseAnswer {
    let npm = registry::npm_newest(at.npm);
    let crates = registry::crates_newest(at.crates);
    let github = match &mine {
        Built::Nightly(_) => registry::nightly_newest(at.github).map(GithubAnswer::Nightly),
        Built::Released(_) | Built::FromSource => {
            registry::github_newest(at.github).map(GithubAnswer::Curated)
        }
    };
    let registries = vec![
        RegistryNewest {
            registry: Registry::Npm,
            reading: reading(npm.as_ref().map(line)),
        },
        RegistryNewest {
            registry: Registry::CratesIo,
            reading: reading(crates.as_ref().map(crates_line)),
        },
        RegistryNewest {
            registry: Registry::Github,
            reading: reading(github.as_ref().map(GithubAnswer::line)),
        },
    ];
    let mine = match mine {
        Built::FromSource
            if matches!(
                installed,
                InstallChannel::Cargo | InstallChannel::Binstall | InstallChannel::CargoOrBinstall
            ) =>
        {
            return match Version::from_crates_version(env!("CARGO_PKG_VERSION"))
                .and_then(|mine| crates.map(|newest| (mine, newest)))
            {
                Ok((mine, newest)) => ReleaseAnswer::Stands {
                    verdict: kernel::release::stands_on_versions(&mine, &newest),
                    mine: crates_line(&mine),
                    newest: crates_line(&newest),
                    registries,
                    update: hint(installed),
                },
                Err(refusal) => ReleaseAnswer::Refused { refusal },
            };
        }
        Built::FromSource => {
            return ReleaseAnswer::Unreleased {
                registries,
                update: hint(installed),
            };
        }
        Built::Nightly(mine) => {
            return at.nightly_judged(mine, installed, github, registries);
        }
        Built::Released(mine) => mine,
    };
    let verdict = match installed {
        InstallChannel::Cargo | InstallChannel::Binstall | InstallChannel::CargoOrBinstall => {
            crates.map(|newest| {
                (
                    kernel::release::stands_on_crates(&mine, &newest),
                    crates_line(&newest),
                    hint(installed),
                )
            })
        }
        InstallChannel::Npm | InstallChannel::Bun | InstallChannel::Package => npm.map(|newest| {
            (
                kernel::release::stands(&mine, &newest),
                line(&newest),
                hint(installed),
            )
        }),
        InstallChannel::Aur => match github {
            Ok(GithubAnswer::Curated(newest)) => Ok((
                kernel::release::stands(&mine, &newest.cut),
                line(&newest.cut),
                hint(installed),
            )),
            Ok(GithubAnswer::Nightly(_)) => {
                return ReleaseAnswer::Unconfirmed {
                    mine: line(&mine),
                    registries,
                    update: hint(installed),
                };
            }
            Err(refusal) => Err(refusal),
        },
        InstallChannel::Archive => match github {
            Ok(GithubAnswer::Curated(newest)) => {
                let mut update = hint(installed);
                update.command = Some(archive_command(ArchiveVersion::Tag(&newest.tag)));
                Ok((
                    kernel::release::stands(&mine, &newest.cut),
                    line(&newest.cut),
                    update,
                ))
            }
            Ok(GithubAnswer::Nightly(_)) => {
                return ReleaseAnswer::Unconfirmed {
                    mine: line(&mine),
                    registries,
                    update: hint(installed),
                };
            }
            Err(refusal) => Err(refusal),
        },
        InstallChannel::Unknown | InstallChannel::Source => {
            return ReleaseAnswer::Unconfirmed {
                mine: line(&mine),
                registries,
                update: hint(InstallChannel::Unknown),
            };
        }
    };
    match verdict {
        Ok((verdict, newest, update)) => ReleaseAnswer::Stands {
            newest,
            verdict,
            mine: line(&mine),
            registries,
            update,
        },
        Err(refusal) => ReleaseAnswer::Refused { refusal },
    }
}

impl Registries<'_> {
    fn nightly_judged(
        &self,
        mine: NightlyBuild,
        installed: InstallChannel,
        github: Result<GithubAnswer, AxError>,
        registries: Vec<RegistryNewest>,
    ) -> ReleaseAnswer {
        let line = |build: &NightlyBuild| ReleaseLine {
            version: build.tag().to_owned(),
            released: String::new(),
        };
        if installed != InstallChannel::Archive {
            return match github {
                Ok(_) => ReleaseAnswer::Unconfirmed {
                    mine: line(&mine),
                    registries,
                    update: hint(installed),
                },
                Err(refusal) => ReleaseAnswer::Refused { refusal },
            };
        }
        match github {
            Ok(GithubAnswer::Nightly(newest)) => {
                let mut update = hint(installed);
                update.command = Some(archive_command(ArchiveVersion::Nightly(&newest)));
                match registry::nightly_standing(self.compare, &mine, &newest) {
                    Ok(Some(verdict)) => ReleaseAnswer::Stands {
                        mine: line(&mine),
                        newest: line(&newest),
                        registries,
                        verdict,
                        update,
                    },
                    Ok(None) => ReleaseAnswer::Unconfirmed {
                        mine: line(&mine),
                        registries,
                        update,
                    },
                    Err(refusal) => ReleaseAnswer::Refused { refusal },
                }
            }
            Ok(GithubAnswer::Curated(_)) => ReleaseAnswer::Unconfirmed {
                mine: line(&mine),
                registries,
                update: hint(installed),
            },
            Err(refusal) => ReleaseAnswer::Refused { refusal },
        }
    }
}

fn reading(read: Result<ReleaseLine, &AxError>) -> RegistryReading {
    match read {
        Ok(newest) => RegistryReading::Read { newest },
        Err(refusal) => RegistryReading::Refused {
            refusal: refusal.clone(),
        },
    }
}

fn this_channel() -> InstallChannel {
    match std::env::var("SPRAWLING_INSTALL_CHANNEL").as_deref() {
        Ok("npm") => return InstallChannel::Npm,
        Ok("bun") => return InstallChannel::Bun,
        Ok("cargo") => return InstallChannel::Cargo,
        Ok("binstall") => return InstallChannel::Binstall,
        Ok("archive") => return InstallChannel::Archive,
        Ok("package") => return InstallChannel::Package,
        Ok("aur") => return InstallChannel::Aur,
        Ok(_) | Err(_) => {}
    }
    let cargo_home = match std::env::var_os("CARGO_HOME") {
        Some(set) => Some(std::path::PathBuf::from(set)),
        None => match accounting::home::Home::detect() {
            Ok(home) => Some(home.path().join(".cargo")),
            Err(_no_home) => None,
        },
    };
    let cargo_bin = cargo_home.map(|cargo_home| {
        let path = cargo_home.join("bin");
        match std::fs::canonicalize(&path) {
            Ok(resolved) => resolved,
            Err(_unresolved_candidate) => path,
        }
    });
    match std::env::current_exe().and_then(std::fs::canonicalize) {
        Ok(exe) => channel(&exe, cargo_bin.as_deref()),
        Err(_unnamed) => InstallChannel::Source,
    }
}

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
    if kernel::release::is_aur_install(exe) {
        InstallChannel::Aur
    } else if packaged {
        InstallChannel::Package
    } else if cargo_bin.is_some() && dir == cargo_bin {
        InstallChannel::CargoOrBinstall
    } else if dir.is_some_and(|dir| {
        dir.join(ARCHIVE_SIBLING).is_dir() || dir.join(ARCHIVE_ORIGIN_FILE).is_file()
    }) {
        InstallChannel::Archive
    } else {
        InstallChannel::Source
    }
}

enum ArchiveVersion<'a> {
    Latest,
    Tag(&'a str),
    Nightly(&'a NightlyBuild),
}

fn archive_command(version: ArchiveVersion<'_>) -> String {
    let repo = RELEASES
        .trim_start_matches("https://github.com/")
        .trim_end_matches("/releases");
    let (reference, set_sh, set_ps) = match version {
        ArchiveVersion::Latest => ("main", String::new(), String::new()),
        ArchiveVersion::Tag(tag) => (
            tag,
            format!("SPRAWLING_VERSION='{tag}' "),
            format!("$env:SPRAWLING_VERSION='{tag}'; "),
        ),
        ArchiveVersion::Nightly(build) => (
            build.tag(),
            "SPRAWLING_RELEASE_CHANNEL=nightly ".to_owned(),
            "$env:SPRAWLING_RELEASE_CHANNEL='nightly'; ".to_owned(),
        ),
    };
    if cfg!(windows) {
        format!(
            "powershell -NoProfile -Command \"{set_ps}irm https://raw.githubusercontent.com/{repo}/{reference}/install.ps1 | iex\""
        )
    } else {
        format!(
            "curl -fsSL https://raw.githubusercontent.com/{repo}/{reference}/install.sh | {set_sh}sh"
        )
    }
}

fn hint(channel: InstallChannel) -> UpdateHint {
    let (command, alternatives) = match channel {
        InstallChannel::Npm => (Some(NPM_UPDATE.to_owned()), Vec::new()),
        InstallChannel::Package => (None, vec![NPM_UPDATE.to_owned(), BUN_UPDATE.to_owned()]),
        InstallChannel::Bun => (Some(BUN_UPDATE.to_owned()), Vec::new()),
        InstallChannel::Cargo => (Some(CARGO_UPDATE.to_owned()), Vec::new()),
        InstallChannel::CargoOrBinstall => (
            None,
            vec![CARGO_UPDATE.to_owned(), BINSTALL_UPDATE.to_owned()],
        ),
        InstallChannel::Binstall => (Some(BINSTALL_UPDATE.to_owned()), Vec::new()),
        InstallChannel::Archive => (Some(archive_command(ArchiveVersion::Latest)), Vec::new()),
        InstallChannel::Aur => (Some(AUR_UPDATE.to_owned()), Vec::new()),
        InstallChannel::Source => (None, Vec::new()),
        InstallChannel::Unknown => (
            None,
            vec![
                NPM_UPDATE.to_owned(),
                BUN_UPDATE.to_owned(),
                CARGO_UPDATE.to_owned(),
                BINSTALL_UPDATE.to_owned(),
                archive_command(ArchiveVersion::Latest),
                AUR_UPDATE.to_owned(),
            ],
        ),
    };
    UpdateHint {
        channel,
        command,
        alternatives,
    }
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
                let body = if head[0].contains("/github") {
                    r#"[{"tag_name":"nightly-261106-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","draft":false,"published_at":"2026-11-06T00:00:00Z","target_commitish":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},{"tag_name":"v0.0.8-Alpha-261006","draft":false}]"#
                } else if head[0].contains("/compare") {
                    r#"{"status":"behind"}"#
                } else if head[0].contains("/crates") {
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
            github: &format!("{base}/github"),
            compare: &format!("{base}/compare"),
        };
        let mine =
            || Built::Released(kernel::Release::from_npm_version("0.0.8-pre.261002").unwrap());
        let verdict = |answer: ReleaseAnswer| match answer {
            ReleaseAnswer::Stands {
                verdict,
                newest,
                update,
                ..
            } => (verdict, newest.version, update.command),
            other => panic!("not judged: {other:?}"),
        };
        assert_eq!(
            [
                verdict(judged(mine(), InstallChannel::Cargo, &at)),
                verdict(judged(mine(), InstallChannel::Npm, &at)),
            ],
            [
                (
                    ReleaseVerdict::Current,
                    "0.0.8".to_owned(),
                    Some(super::CARGO_UPDATE.to_owned())
                ),
                (
                    ReleaseVerdict::Behind,
                    "0.0.8-pre.261005".to_owned(),
                    Some(super::NPM_UPDATE.to_owned())
                ),
            ]
        );
        let agents: Vec<String> = seen.try_iter().collect();
        assert_eq!(agents.len(), 6);
        assert!(
            agents.iter().all(|agent| agent.starts_with("sprawling/")),
            "{agents:?}"
        );
    }

    #[test]
    fn each_origin_selects_its_own_registry_and_never_switches_updaters() {
        let (base, _seen) = registries(
            r#"{"version":"0.0.9-pre.261005"}"#,
            r#"{"crate":{"max_version":"1.0.0"}}"#,
        );
        let at = Registries {
            npm: &format!("{base}/npm"),
            crates: &format!("{base}/crates"),
            github: &format!("{base}/github"),
            compare: &format!("{base}/compare"),
        };
        let mine =
            || Built::Released(kernel::Release::from_npm_version("0.0.8-pre.261002").unwrap());
        let read = |origin| match judged(mine(), origin, &at) {
            ReleaseAnswer::Stands { newest, update, .. } => {
                (newest.version, update.command, update.alternatives)
            }
            other => panic!("not judged: {other:?}"),
        };
        assert_eq!(
            [
                InstallChannel::Npm,
                InstallChannel::Bun,
                InstallChannel::Binstall,
                InstallChannel::Archive,
                InstallChannel::CargoOrBinstall,
                InstallChannel::Package,
                InstallChannel::Aur
            ]
            .map(read),
            [
                (
                    "0.0.9-pre.261005".to_owned(),
                    Some(super::NPM_UPDATE.to_owned()),
                    vec![]
                ),
                (
                    "0.0.9-pre.261005".to_owned(),
                    Some(super::BUN_UPDATE.to_owned()),
                    vec![]
                ),
                (
                    "1.0.0".to_owned(),
                    Some(super::BINSTALL_UPDATE.to_owned()),
                    vec![]
                ),
                (
                    "0.0.8-pre.261006".to_owned(),
                    Some(super::archive_command(super::ArchiveVersion::Tag(
                        "v0.0.8-Alpha-261006"
                    ))),
                    vec![]
                ),
                (
                    "1.0.0".to_owned(),
                    None,
                    vec![
                        super::CARGO_UPDATE.to_owned(),
                        super::BINSTALL_UPDATE.to_owned()
                    ]
                ),
                (
                    "0.0.9-pre.261005".to_owned(),
                    None,
                    vec![super::NPM_UPDATE.to_owned(), super::BUN_UPDATE.to_owned()]
                ),
                (
                    "0.0.8-pre.261006".to_owned(),
                    Some(super::AUR_UPDATE.to_owned()),
                    vec![]
                ),
            ]
        );
        assert!(matches!(
            judged(mine(), InstallChannel::Unknown, &at),
            ReleaseAnswer::Unconfirmed {
                update: wire::UpdateHint { command: None, .. },
                ..
            }
        ));
        assert!(
            matches!(judged(Built::FromSource, InstallChannel::CargoOrBinstall, &at), ReleaseAnswer::Stands { newest, mine, .. } if newest.version == "1.0.0" && mine.released.is_empty())
        );
    }

    #[test]
    fn a_nightly_archive_compares_complete_source_and_keeps_its_channel() {
        let (base, _seen) = registries(
            r#"{"version":"0.0.9-pre.261005"}"#,
            r#"{"crate":{"max_version":"1.0.0"}}"#,
        );
        let at = Registries {
            npm: &format!("{base}/npm"),
            crates: &format!("{base}/crates"),
            github: &format!("{base}/github"),
            compare: &format!("{base}/compare"),
        };
        let mine = Built::Nightly(
            super::NightlyBuild::new(
                "nightly-261105-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
        );
        let newest = super::NightlyBuild::new(
            "nightly-261106-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        match judged(mine, InstallChannel::Archive, &at) {
            ReleaseAnswer::Stands {
                verdict,
                newest: found,
                update,
                ..
            } => assert_eq!(
                (verdict, found.version, update.command),
                (
                    ReleaseVerdict::Behind,
                    "nightly-261106-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
                    Some(super::archive_command(super::ArchiveVersion::Nightly(
                        &newest
                    )))
                )
            ),
            other => panic!("not judged: {other:?}"),
        }
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
                InstallChannel::Package,
                InstallChannel::Package,
                InstallChannel::CargoOrBinstall,
                InstallChannel::Archive,
                InstallChannel::Source,
                InstallChannel::Source,
            ]
        );
    }

    #[test]
    fn system_package_paths_select_their_own_update_guidance() {
        let root = tempfile::tempdir().unwrap();
        let aur = root
            .path()
            .join(kernel::release::aur_install_directory())
            .join("sprawling");
        assert_eq!(channel(&aur, None), InstallChannel::Aur);
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
            Built::Nightly(mine) => {
                assert!(mine.tag().starts_with("nightly-"));
                assert_eq!(mine.source_sha().len(), 40);
            }
        }
    }
}
