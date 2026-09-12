// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The npm channel: the archives' own binaries, reachable by `bunx`.
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
//! and never calls `sprawling install` — see `npm/shim.js`.
//!
//! *The platform list comes from the assets.* An archive with no row
//! here is refused rather than skipped: the day a Linux archive returns,
//! this says so instead of publishing a channel that quietly lacks it.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::report::XtaskError;

/// The shim, compiled in so the file a reader opens and the file a
/// package carries are the same bytes — the rule `docs/templates/`
/// already follows for the documents a city writes.
const SHIM: &str = include_str!("../../npm/shim.js");

/// How an archive's platform suffix becomes an npm package.
///
/// `os` and `cpu` are npm's own spellings, and they are what make the
/// install conditional: npm and bun skip an optional dependency whose
/// `os`/`cpu` do not match, so one machine downloads one binary.
struct Row {
    /// What the release archive's name ends with.
    suffix: &'static str,
    package: &'static str,
    os: &'static str,
    cpu: &'static str,
    binary: &'static str,
}

/// Every platform the channel can carry. The Linux row is here and its
/// archive is not: `release.yml` holds that build back until it is
/// settled where a Linux install keeps a credential. A release without
/// that asset simply publishes two packages; a release *with* an asset
/// this table does not know is an error, which is the asymmetry that
/// keeps a new platform from being dropped in silence.
///
/// **These names carry the `@sprawling` scope, and the versions at or
/// below `UNSCOPED_THROUGH` do not.** npm never reuses a `name@version`,
/// so the rename cannot reach backwards: everything already on the
/// registry under a bare name stays there and is deprecated in place,
/// pointing at the scoped name that succeeds it. The root package keeps
/// its bare name whatever happens to these: `bunx sprawling` is the
/// whole reason this channel exists, and a scoped root would spell it
/// `bunx @sprawling/sprawling`. A scoped name writes one directory level
/// more than a bare one, which is why the publish step enumerates
/// `target/npm/@sprawling/*/` rather than every child of `target/npm`.
const ROWS: [Row; 3] = [
    Row {
        suffix: "-windows-x86_64.zip",
        package: "@sprawling/sprawling-windows-x64",
        os: "win32",
        cpu: "x64",
        binary: "sprawling.exe",
    },
    Row {
        suffix: "-macos-aarch64.zip",
        package: "@sprawling/sprawling-darwin-arm64",
        os: "darwin",
        cpu: "arm64",
        binary: "sprawling",
    },
    Row {
        suffix: "-x86_64-unknown-linux-musl.zip",
        package: "@sprawling/sprawling-linux-x64-musl",
        os: "linux",
        cpu: "x64",
        binary: "sprawling",
    },
];

const REPOSITORY: &str = "https://github.com/2youg1/sprawling-agents";

/// One file's bytes, out of a zip that holds it at any depth.
fn extract(archive: &Path, wanted: &str) -> Result<Vec<u8>, XtaskError> {
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
            return Ok(bytes);
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

/// The manifest shared by every package here: the same licence, the same
/// repository, the same description stem — stated once so two packages
/// cannot disagree about what this project is.
fn manifest(name: &str, version: &str, description: &str, extra: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "{version}",
  "description": "{description}",
  "license": "MPL-2.0",
  "repository": {{ "type": "git", "url": "git+{REPOSITORY}.git" }},
  "homepage": "{REPOSITORY}",
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
    let workspace = crate::package::workspace_version(root)?;
    // `kernel::Release` owns both spellings of one release, because the
    // binary this channel packages has to recognise on the registry the
    // same release this job publishes there.
    let version = kernel::Release::from_tag(tag, &workspace)
        .map_err(|err| XtaskError::Cmd {
            cmd: format!("npm version for {tag}"),
            msg: err.recovery().to_owned(),
        })?
        .npm_version();

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

    let mut carried: Vec<&Row> = Vec::new();
    for archive in &archives {
        let name = archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let row = ROWS.iter().find(|row| name.ends_with(row.suffix));
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
        let binary = extract(archive, row.binary)?;
        let dir = out.join(row.package);
        write(&dir.join("bin").join(row.binary), &binary)?;
        let extra = format!(
            "  \"os\": [\"{}\"],\n  \"cpu\": [\"{}\"],\n  \"files\": [\"bin\"]\n",
            row.os, row.cpu
        );
        let description = format!("The sprawling binary for {} {}.", row.os, row.cpu);
        write(
            &dir.join("package.json"),
            manifest(row.package, &version, &description, &extra).as_bytes(),
        )?;
        carried.push(row);
    }

    if carried.is_empty() {
        return Err(XtaskError::Cmd {
            cmd: String::from("npm channel"),
            msg: format!("{} holds no release archive", assets.display()),
        });
    }

    let optional = carried
        .iter()
        .map(|row| format!("    \"{}\": \"{version}\"", row.package))
        .collect::<Vec<_>>()
        .join(",\n");
    let extra = format!(
        "  \"bin\": {{ \"sprawling\": \"bin/sprawling.js\" }},\n  \
         \"files\": [\"bin\"],\n  \
         \"optionalDependencies\": {{\n{optional}\n  }}\n"
    );
    let dir = out.join("sprawling");
    write(
        &dir.join("package.json"),
        manifest(
            "sprawling",
            &version,
            "Raise a city of agents that work on your repository.",
            &extra,
        )
        .as_bytes(),
    )?;
    write(&dir.join("bin").join("sprawling.js"), SHIM.as_bytes())?;

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
