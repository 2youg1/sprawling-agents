// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Release archives projected into npm and AUR packages (xtask D32).
//!
//! **The archives are the artefact, and this repackages them.** It reads
//! the zips a tag published, takes the binary out of each, and writes
//! one npm package per platform plus a root package that depends on all
//! of them optionally. Nothing is compiled here: a binary that reached a
//! person through npm and a binary that reached them through a download
//! are the same bytes, or one of the two is unaccounted for.
//!
//! **Three rules this channel is built around.**
//!
//! *The version is derived, never written.* npm accepts semver and
//! nothing else, so `v0.0.4-Pre-alpha-260911` cannot be published under
//! its own name. It becomes `0.0.4-pre.260911`, out of the workspace
//! version and the tag's date — and the tag's own version has to agree
//! with the workspace's, or this refuses. A hand-written npm version
//! would be a second answer to "which release is this".
//!
//! That conversion lives in `kernel::Release` rather than here, because
//! the binary being packaged reads it too: `sprawling status --check`
//! asks the registry which release is newest, and it can only compare
//! that answer with itself if both ends spell a release the same way.
//!
//! *PATH belongs to whoever installed.* `sprawling install` owns the
//! archive path; npm and bun own theirs. The shim resolves and execs,
//! and never calls `sprawling install` — see `tools/xtask/src/channel/shim.js`.
//!
//! *The platform list comes from the assets.* An archive with no row in
//! `xtask::platform` is refused rather than skipped: the day a Linux
//! archive returns, this says so instead of publishing a channel that
//! quietly lacks it.

use sha2::{Digest as _, Sha256};
use std::io::Read as _;

use std::path::{Path, PathBuf};

use crate::platform::{PLATFORMS, Platform, ROOT_PACKAGE};
use crate::report::XtaskError;

mod system;

/// The shim, compiled in so the file a reader opens and the file a
/// package carries are the same bytes — the rule `crates/city/templates/`
/// already follows for the documents a city writes.
const SHIM: &str = include_str!("channel/shim.js");
const RUNTIME_ENTRY: &str = "sprawling-runtime";
const LAUNCHER: &str = include_str!("channel/launcher.cmd");

/// One file's bytes, out of a zip that holds it at any depth.
struct ArchiveBinary {
    bytes: Vec<u8>,
    mode: u32,
}

fn extract(archive: &Path, wanted: &str) -> Result<ArchiveBinary, XtaskError> {
    let file = std::fs::File::open(archive).map_err(|source| XtaskError::Io {
        path: archive.display().to_string(),
        source,
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|err| XtaskError::Cmd {
        cmd: format!("read {}", archive.display()),
        msg: err.to_string(),
    })?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|err| XtaskError::Cmd {
            cmd: format!("read {}", archive.display()),
            msg: err.to_string(),
        })?;
        // The archive holds `sprawling-<version>-<platform>/<name>`, and
        // the directory carries the version, so the entry is matched on
        // its file name rather than on a path this would have to rebuild.
        let is_wanted = entry.name().rsplit('/').next() == Some(wanted);
        if is_wanted {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|source| XtaskError::Io {
                    path: archive.display().to_string(),
                    source,
                })?;
            let mode = entry.unix_mode().ok_or_else(|| XtaskError::Cmd {
                cmd: format!("read {}", archive.display()),
                msg: format!("{wanted} carries no executable permissions"),
            })?;
            return Ok(ArchiveBinary { bytes, mode });
        }
    }
    Err(XtaskError::Cmd {
        cmd: format!("read {}", archive.display()),
        msg: format!("holds no file named {wanted}"),
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), XtaskError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    std::fs::write(path, bytes).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// What every package of the channel states alike: which release it is,
/// and where its source lives. Both are read out of the workspace
/// manifest, the one place either is written.
struct Stem<'a> {
    version: &'a str,
    repository: &'a str,
}

/// The manifest shared by every package here: the same licence, the same
/// repository, the same description stem — stated once so two packages
/// cannot disagree about what this project is.
fn manifest(stem: &Stem<'_>, name: &str, description: &str, extra: &str) -> String {
    let Stem {
        version,
        repository,
    } = stem;
    format!(
        r#"{{
  "name": "{name}",
  "version": "{version}",
  "description": "{description}",
  "license": "MPL-2.0",
  "repository": {{ "type": "git", "url": "git+{repository}.git" }},
  "homepage": "{repository}",
{extra}}}
"#
    )
}

/// Assemble the channel from a directory of release archives.
///
/// # Errors
/// When the tag is misshapen, when an archive holds no binary, when an
/// archive matches no row, or when the output cannot be written.
pub(crate) fn run(root: &Path, tag: &str, assets: &Path, out: &Path) -> Result<String, XtaskError> {
    let workspace = crate::package::workspace_package(root, "version")?;
    let repository = crate::package::workspace_package(root, "repository")?;
    let description = crate::package::workspace_package(root, "description")?;
    // `kernel::Release` owns both spellings of one release, because the
    // binary this channel packages has to recognise on the registry the
    // same release this job publishes there.
    let version = kernel::Release::from_tag(tag, &workspace)
        .map_err(|err| XtaskError::Cmd {
            cmd: format!("npm version for {tag}"),
            msg: err.recovery().to_owned(),
        })?
        .npm_version();
    let stem = Stem {
        version: &version,
        repository: &repository,
    };

    let entries = std::fs::read_dir(assets).map_err(|source| XtaskError::Io {
        path: assets.display().to_string(),
        source,
    })?;
    let mut archives: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| XtaskError::Io {
            path: assets.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "zip") {
            archives.push(path);
        }
    }
    archives.sort();

    let mut carried: Vec<&Platform> = Vec::new();
    let mut system_archives = Vec::new();
    for archive in &archives {
        let name = archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let row = PLATFORMS.iter().find(|row| name.ends_with(row.suffix));
        let Some(row) = row else {
            return Err(XtaskError::Cmd {
                cmd: String::from("npm channel"),
                msg: format!(
                    "{name} matches no platform this channel knows. \
                     Add its row beside the others, or the release publishes \
                     a channel that silently lacks a platform it built."
                ),
            });
        };
        let sha256 = Sha256::digest(std::fs::read(archive).map_err(|source| XtaskError::Io {
            path: archive.display().to_string(),
            source,
        })?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .concat();
        system_archives.push(system::Archive {
            platform: row,
            name,
            sha256,
        });
        let binary = extract(archive, row.binary)?;
        // A scoped npm name writes one directory level more than a bare
        // one, which is why the publish step enumerates
        // `target/npm/@sprawling/*/` rather than every child of `target/npm`.
        let dir = out.join(row.package);
        let path = dir.join("bin").join(row.binary);
        if binary.mode & 0o111 == 0 {
            return Err(XtaskError::Cmd {
                cmd: "npm channel".to_owned(),
                msg: format!("{} has no executable permission bits", archive.display()),
            });
        }
        write(&path, &binary.bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(binary.mode)).map_err(
                |source| XtaskError::Io {
                    path: path.display().to_string(),
                    source,
                },
            )?;
        }
        let extra = format!(
            "  \"os\": [\"{}\"],\n  \"cpu\": [\"{}\"],\n  \"files\": [\"bin\"]\n",
            row.os, row.cpu
        );
        let description = format!("The sprawling binary for {} {}.", row.os, row.cpu);
        write(
            &dir.join("package.json"),
            manifest(&stem, row.package, &description, &extra).as_bytes(),
        )?;
        carried.push(row);
    }

    if carried.is_empty() {
        return Err(XtaskError::Cmd {
            cmd: String::from("npm channel"),
            msg: format!("{} holds no release archive", assets.display()),
        });
    }

    system::write(&stem, tag, &system_archives, out)?;

    let optional = carried
        .iter()
        .map(|row| format!("    \"{}\": \"{version}\"", row.package))
        .collect::<Vec<_>>()
        .join(",\n");
    let extra = format!(
        "  \"bin\": {{ \"{ROOT_PACKAGE}\": \"bin/sprawling.js\", \"{RUNTIME_ENTRY}\": \"bin/sprawling.cmd\" }},\n  \
         \"files\": [\"bin\"],\n  \
         \"optionalDependencies\": {{\n{optional}\n  }}\n"
    );
    let dir = out.join(ROOT_PACKAGE);
    write(
        &dir.join("package.json"),
        manifest(&stem, ROOT_PACKAGE, &description, &extra).as_bytes(),
    )?;
    write(
        &dir.join("bin").join("sprawling.js"),
        format!(
            "#!/usr/bin/env {RUNTIME_ENTRY}
{SHIM}"
        )
        .as_bytes(),
    )?;
    write(&dir.join("bin").join("sprawling.cmd"), LAUNCHER.as_bytes())?;

    let names = carried
        .iter()
        .map(|row| row.package)
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "npm channel {version} written to {}: sprawling + {names}",
        out.display()
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
