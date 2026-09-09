// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The deliverable: the one archive a person downloads, unpacks and runs.
//!
//! A release somebody assembled by hand is a release nobody can check.
//! This assembles the same archive on every platform, out of artifacts the
//! gates have already weighed, and refuses a binary whose client is only
//! the page shell - the defect that survives a green build and reaches the
//! person as an empty browser window.
//!
//! Entry timestamps are fixed rather than taken from the file system, so
//! two builds of one tree produce the same archive bytes.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::report::XtaskError;

/// Which build an archive is assembled from. Exhaustive on purpose: the
/// two questions a packager asks — where the binary landed, and what the
/// archive is called — have one answer each per variant, and a boolean
/// would have left the second one to the host it happened to run on.
pub(crate) enum ReleaseTarget {
    /// Built for this machine's own default target.
    Host,
    /// Built with `--target <triple>`, which lands elsewhere in `target/`.
    Triple(String),
}

impl ReleaseTarget {
    /// The triple named on the command line, or the host's own build.
    pub(crate) fn from_arg(triple: Option<&str>) -> Self {
        match triple {
            Some(named) if !named.is_empty() => Self::Triple(named.to_owned()),
            _ => Self::Host,
        }
    }

    /// What a person recognises in a list of assets. The host's two names
    /// are what they always were; a build for anything else carries its
    /// triple, because `linux-x86_64` says neither static nor musl and
    /// would collide the day a gnu Linux artifact appears.
    fn label(&self) -> String {
        match self {
            Self::Host => format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            Self::Triple(triple) => triple.clone(),
        }
    }

    /// The directory `cargo build --release` puts the binary in.
    fn release_dir(&self, root: &Path) -> PathBuf {
        match self {
            Self::Host => root.join("target").join("release"),
            Self::Triple(triple) => root.join("target").join(triple).join("release"),
        }
    }

    /// What the executable is called on the system it will run on, which
    /// is the target's question rather than the assembling machine's.
    fn binary_name(&self) -> &'static str {
        let windows = match self {
            Self::Host => cfg!(windows),
            Self::Triple(triple) => triple.contains("windows"),
        };
        if windows {
            "sprawling.exe"
        } else {
            "sprawling"
        }
    }
}

/// Where a release build of this target landed, or nothing when it has
/// not been built. One statement of that path, because the packager
/// assembles from it and the budget gate weighs it.
pub(crate) fn binary_path(root: &Path, target: &ReleaseTarget) -> Option<PathBuf> {
    let dir = target.release_dir(root);
    for name in ["sprawling", "sprawling.exe"] {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn run(root: &Path, target: &ReleaseTarget) -> Result<String, XtaskError> {
    let binary = binary_path(root, target).ok_or_else(|| XtaskError::Cmd {
        cmd: "package".to_owned(),
        msg: format!(
            "no release binary in {}; run `just dist` first",
            target.release_dir(root).display()
        ),
    })?;
    if !crate::budget::carries_client(&binary)? {
        return Err(XtaskError::Cmd {
            cmd: "package".to_owned(),
            msg: "this binary carries the page shell only and would serve an empty page; \
                  run `just build-web`, then `just dist`"
                .to_owned(),
        });
    }

    let stem = format!("sprawling-{}-{}", version(root)?, target.label());
    let out_dir = root.join("target").join("package");
    std::fs::create_dir_all(&out_dir).map_err(|source| XtaskError::Io {
        path: out_dir.display().to_string(),
        source,
    })?;
    let archive = out_dir.join(format!("{stem}.zip"));

    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    entries.push((target.binary_name().to_owned(), binary));
    entries.push((
        "QUICKSTART.md".to_owned(),
        root.join("dist").join("QUICKSTART.md"),
    ));
    entries.push(("LICENSE".to_owned(), root.join("LICENSE")));
    // The bill of materials when `just dist` produced one; a person who
    // wants to know what is inside the binary should not have to build it.
    let sbom = root.join("target").join("sbom.cdx.json");
    if sbom.is_file() {
        entries.push(("sbom.cdx.json".to_owned(), sbom));
    }

    write_archive(&archive, &stem, &entries)?;
    let size = std::fs::metadata(&archive)
        .map(|meta| meta.len())
        .map_err(|source| XtaskError::Io {
            path: archive.display().to_string(),
            source,
        })?;
    Ok(format!(
        "packaged {} ({size} bytes, {} entries)\n",
        archive.display(),
        entries.len()
    ))
}

/// Writes the archive with every entry under one directory, so unpacking
/// it produces a folder rather than scattering files where it landed.
fn write_archive(
    archive: &Path,
    stem: &str,
    entries: &[(String, PathBuf)],
) -> Result<(), XtaskError> {
    let file = std::fs::File::create(archive).map_err(|source| XtaskError::Io {
        path: archive.display().to_string(),
        source,
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let io = |path: &Path, source: std::io::Error| XtaskError::Io {
        path: path.display().to_string(),
        source,
    };
    for (name, source) in entries {
        let bytes = std::fs::read(source).map_err(|err| io(source, err))?;
        // Executable bits: the archive is the only thing that carries them
        // to a machine that has never seen this file, and a binary that
        // arrives without them is a binary nobody can run.
        let mode = if name == "sprawling" { 0o755 } else { 0o644 };
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(mode);
        zip.start_file(format!("{stem}/{name}"), options)
            .map_err(|err| XtaskError::Cmd {
                cmd: "package".to_owned(),
                msg: format!("{name}: {err}"),
            })?;
        zip.write_all(&bytes).map_err(|err| io(archive, err))?;
    }
    zip.finish().map_err(|err| XtaskError::Cmd {
        cmd: "package".to_owned(),
        msg: err.to_string(),
    })?;
    Ok(())
}

/// The workspace version, read from the manifest that defines it rather
/// than from this tool's own compiled-in copy.
fn version(root: &Path) -> Result<String, XtaskError> {
    let path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: err.to_string(),
    })?;
    parsed
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: path.display().to_string(),
            msg: "workspace.package.version is missing".to_owned(),
        })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::{ReleaseTarget, binary_path, write_archive};

    /// The two names that already exist must not move; a build for a
    /// target other than this machine's default earns its triple in the
    /// name, so a second Linux artifact cannot collide with the first.
    #[test]
    fn a_host_archive_keeps_its_name_and_a_target_archive_carries_its_triple() {
        let host = ReleaseTarget::Host.label();
        assert!(host.contains('-'), "host label: {host}");
        assert!(!host.starts_with('-'), "host label: {host}");
        let musl = ReleaseTarget::Triple("x86_64-unknown-linux-musl".to_owned());
        assert_eq!(musl.label(), "x86_64-unknown-linux-musl");
    }

    /// A `--target` build lands under `target/<triple>/release`, and the
    /// packager must look there rather than at the host's directory.
    #[test]
    fn a_target_build_is_looked_for_under_its_own_triple() {
        let root = std::env::temp_dir().join(format!("sprawling-triple-{}", std::process::id()));
        let triple = "x86_64-unknown-linux-musl";
        let dir = root.join("target").join(triple).join("release");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sprawling"), b"binary").unwrap();

        let target = ReleaseTarget::Triple(triple.to_owned());
        assert_eq!(binary_path(&root, &target), Some(dir.join("sprawling")));
        assert_eq!(
            binary_path(&root, &ReleaseTarget::Host),
            None,
            "the host directory holds nothing and must not answer for the triple"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// What the archive calls the executable follows the target it was
    /// built for, never the machine that assembled it.
    #[test]
    fn the_executable_name_follows_the_target_rather_than_the_host() {
        let windows = ReleaseTarget::Triple("x86_64-pc-windows-msvc".to_owned());
        assert_eq!(windows.binary_name(), "sprawling.exe");
        let musl = ReleaseTarget::Triple("x86_64-unknown-linux-musl".to_owned());
        assert_eq!(musl.binary_name(), "sprawling");
    }

    /// The archive nests under one directory: unpacking it into a folder
    /// full of other things must not scatter six files across it.
    #[test]
    fn every_entry_sits_under_one_directory() {
        let dir = std::env::temp_dir().join("sprawling-package-test");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("payload.txt");
        std::fs::write(&source, b"payload").unwrap();
        let archive = dir.join("out.zip");
        write_archive(
            &archive,
            "sprawling-9.9.9-test",
            &[("QUICKSTART.md".to_owned(), source)],
        )
        .unwrap();

        let bytes = std::fs::read(&archive).unwrap();
        let listing = String::from_utf8_lossy(&bytes);
        assert!(
            listing.contains("sprawling-9.9.9-test/QUICKSTART.md"),
            "entry is not under the archive's own directory"
        );
    }

    /// Two runs over one tree produce one archive: a release that differs
    /// from its own rebuild cannot be checked against its source.
    #[test]
    fn two_runs_over_the_same_files_produce_the_same_bytes() {
        let dir = std::env::temp_dir().join("sprawling-package-repro");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("payload.txt");
        std::fs::write(&source, b"payload").unwrap();
        let entries = [("QUICKSTART.md".to_owned(), source)];

        let first = dir.join("first.zip");
        let second = dir.join("second.zip");
        write_archive(&first, "sprawling-9.9.9-test", &entries).unwrap();
        write_archive(&second, "sprawling-9.9.9-test", &entries).unwrap();
        assert_eq!(
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap()
        );
    }
}
