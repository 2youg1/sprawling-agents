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
/// **Unscoped names, and that was decided by the registry.** A scope
/// belongs to a user or an organisation, and publishing into one nobody
/// owns is answered with a 404 that reads as if the package were
/// missing rather than as if the scope were. `sprawling-windows-x64`
/// needs no organisation to exist first, and it keeps the platform
/// packages sorted next to the root package they belong to. Moving to a
/// scope later is a change to this table and to the shim's, which one
/// test already holds together.
const ROWS: [Row; 3] = [
    Row {
        suffix: "-windows-x86_64.zip",
        package: "sprawling-windows-x64",
        os: "win32",
        cpu: "x64",
        binary: "sprawling.exe",
    },
    Row {
        suffix: "-macos-aarch64.zip",
        package: "sprawling-darwin-arm64",
        os: "darwin",
        cpu: "arm64",
        binary: "sprawling",
    },
    Row {
        suffix: "-x86_64-unknown-linux-musl.zip",
        package: "sprawling-linux-x64-musl",
        os: "linux",
        cpu: "x64",
        binary: "sprawling",
    },
];

const REPOSITORY: &str = "https://github.com/2youg1/sprawling-agents";

/// `v0.0.4-Pre-alpha-260911` and `0.0.4` become `0.0.4-pre.260911`.
///
/// # Errors
/// When the tag is not shaped like a release of this project, or when
/// its version disagrees with the workspace's.
fn npm_version(tag: &str, workspace: &str) -> Result<String, XtaskError> {
    let refused = |msg: String| XtaskError::Cmd {
        cmd: format!("npm version for {tag}"),
        msg,
    };
    let body = tag.strip_prefix('v').ok_or_else(|| {
        refused(String::from(
            "a release tag begins with `v`, as in `v0.0.4-Pre-alpha-260911`",
        ))
    })?;
    let (version, rest) = body.split_once("-Pre-alpha-").ok_or_else(|| {
        refused(String::from(
            "a release tag reads `v<version>-Pre-alpha-<YYMMDD>`; \
             a tag of another shape needs this rule written for it",
        ))
    })?;
    if version != workspace {
        return Err(refused(format!(
            "the tag says {version} and `workspace.package.version` says {workspace}; \
             one release cannot have two version numbers"
        )));
    }
    if rest.len() != 6 || !rest.chars().all(|c| c.is_ascii_digit()) {
        return Err(refused(format!(
            "the date in the tag reads `{rest}`, which is not six digits"
        )));
    }
    // `-pre.<date>` rather than `-Pre-alpha-<date>`: semver's pre-release
    // field is dot-separated alphanumerics, and it orders below the
    // release of the same number, which is what a pre-alpha should do.
    Ok(format!("{workspace}-pre.{rest}"))
}

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
    let version = npm_version(tag, &workspace)?;

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
mod tests {
    use super::{ROWS, npm_version};

    #[test]
    fn a_release_tag_becomes_the_semver_npm_accepts() {
        assert_eq!(
            npm_version("v0.0.4-Pre-alpha-260911", "0.0.4").unwrap(),
            "0.0.4-pre.260911"
        );
    }

    /// The check that keeps one release from having two version numbers.
    #[test]
    fn a_tag_disagreeing_with_the_workspace_is_refused() {
        let err = npm_version("v0.0.3-Pre-alpha-260911", "0.0.4").unwrap_err();
        assert!(err.to_string().contains("two version numbers"), "{err}");
    }

    #[test]
    fn a_tag_of_another_shape_is_refused_rather_than_guessed_at() {
        assert!(npm_version("0.0.4", "0.0.4").is_err());
        assert!(npm_version("v0.0.4", "0.0.4").is_err());
        assert!(npm_version("v0.0.4-Pre-alpha-26091", "0.0.4").is_err());
        assert!(npm_version("v0.0.4-Pre-alpha-2609xx", "0.0.4").is_err());
    }

    /// Two packages claiming one platform would make which binary a
    /// person gets depend on the order the assets were read in.
    #[test]
    fn no_two_rows_claim_one_platform_or_one_suffix() {
        for (index, row) in ROWS.iter().enumerate() {
            for other in ROWS.iter().skip(index + 1) {
                assert_ne!((row.os, row.cpu), (other.os, other.cpu));
                assert_ne!(row.suffix, other.suffix);
                assert_ne!(row.package, other.package);
            }
        }
    }

    /// Found by running it rather than by reading it: npm's generated
    /// wrapper reads the shebang to decide what interprets the file, and
    /// without one Windows ran the JavaScript as a shell script, printed
    /// nothing, and exited 0. A `bunx sprawling` that reports success
    /// and does nothing is the worst shape this channel can fail in, so
    /// the first line is held here.
    #[test]
    fn the_shim_begins_with_a_shebang() {
        assert!(
            super::SHIM.starts_with("#!/usr/bin/env node\n"),
            "without a shebang npm's wrapper runs this as a shell script"
        );
    }

    /// The shim is what every root package carries, so its own contract
    /// is worth holding: it resolves a platform package and never
    /// installs anything.
    #[test]
    fn the_shim_execs_rather_than_installs() {
        let shim = super::SHIM;
        assert!(shim.contains("require.resolve"), "the shim must resolve");
        assert!(
            !shim.contains("sprawling install"),
            "PATH belongs to whoever installed; the shim must not install"
        );
        for row in ROWS {
            assert!(
                shim.contains(row.package),
                "the shim does not know {}",
                row.package
            );
        }
    }
}
