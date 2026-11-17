// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! Manual checks: install origin selects the registry, comparison operand and updater.
//! Specified by `Sprawling/wire/spec/Answer/Release.lean` and `Sprawling/app/spec/Install.lean`.
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
/// Refuses a tag that disagrees with the Cargo version.
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
mod tests;
