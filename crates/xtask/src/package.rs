// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::platform;
use crate::report::XtaskError;

mod contents;

use contents::{Entry, Executable};

pub(crate) enum ReleaseTarget {
    Host,
    Triple(String),
}

impl ReleaseTarget {
    pub(crate) fn from_arg(triple: Option<&str>) -> Self {
        match triple {
            Some(named) if !named.is_empty() => Self::Triple(named.to_owned()),
            _ => Self::Host,
        }
    }

    fn label(&self) -> String {
        match self {
            Self::Host => format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            Self::Triple(triple) => triple.clone(),
        }
    }

    fn release_dir(&self, root: &Path) -> PathBuf {
        self.release_dir_in(&cargo_target_dir(
            root,
            std::env::var_os("CARGO_TARGET_DIR"),
        ))
    }

    fn release_dir_in(&self, target: &Path) -> PathBuf {
        match self {
            Self::Host => target.join("release"),
            Self::Triple(triple) => target.join(triple).join("release"),
        }
    }

    fn binary_name(&self) -> Result<&'static str, XtaskError> {
        Ok(self.platform()?.binary)
    }

    fn platform(&self) -> Result<&'static platform::Platform, XtaskError> {
        let suffix = format!("-{}.zip", self.label());
        platform::with_suffix(&suffix).ok_or_else(|| XtaskError::Cmd {
            cmd: "package".to_owned(),
            msg: format!(
                "no platform this release publishes for ends an archive with `{suffix}`; \
                 add the row to crates/xtask/src/platform.rs, and the matrix and install scripts \
                 that restate it, in one change-set"
            ),
        })
    }
}

fn cargo_target_dir(root: &Path, named: Option<std::ffi::OsString>) -> PathBuf {
    match named.map(PathBuf::from) {
        Some(stated) if stated.as_os_str().is_empty() => root.join("target"),
        Some(stated) => root.join(stated),
        None => root.join("target"),
    }
}

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
    if !crate::budget::carries_engine(&binary)? {
        return Err(XtaskError::Cmd {
            cmd: "package".to_owned(),
            msg: "this binary carries no execution engine, so every `python` call in the \
                  archive would be refused; build the release with the engine (the \
                  `sandbox` feature of the `sprawling` package), then `just dist`"
                .to_owned(),
        });
    }

    let stem = format!(
        "sprawling-{}-{}",
        workspace_package(root, "version")?,
        target.label()
    );
    let entries = contents::entries(
        root,
        &Executable {
            name: target.binary_name()?,
            path: &binary,
        },
    )?;
    assemble(root, &stem, &entries)
}

pub(crate) fn run_mei(root: &Path, target: &ReleaseTarget) -> Result<String, XtaskError> {
    let row = target.platform()?;
    let Some(_) = platform::mei_package(row) else {
        return Err(XtaskError::Cmd {
            cmd: "package --mei".to_owned(),
            msg: "Mei publishes no archive for this platform; build it for a row \
                  `mei_package` names (Windows x64, macOS arm64, or Linux x64 musl)"
                .to_owned(),
        });
    };
    let archive_name = platform::mei_binary(row);
    let face_name = platform::calyx_binary(row);
    let binary = target.release_dir(root).join(archive_name);
    let face = target.release_dir(root).join(face_name);
    if !binary.is_file() {
        return Err(XtaskError::Cmd {
            cmd: "package --mei".to_owned(),
            msg: format!(
                "no Mei release binary `{archive_name}` in {}; run `just dist-mei` first",
                target.release_dir(root).display()
            ),
        });
    }
    if !face.is_file() {
        return Err(XtaskError::Cmd {
            cmd: "package --mei".to_owned(),
            msg: format!(
                "no Calyx release binary `{face_name}` in {}; run `just dist-mei` first",
                target.release_dir(root).display()
            ),
        });
    }
    let stem = format!(
        "{}{}-{}",
        platform::MEI_ARCHIVE_PREFIX,
        workspace_package(root, "version")?,
        target.label()
    );
    let entries = [
        Entry {
            name: archive_name.to_owned(),
            source: binary,
            mode: 0o755,
        },
        Entry {
            name: face_name.to_owned(),
            source: face,
            mode: 0o755,
        },
        Entry {
            name: "README.md".to_owned(),
            source: root.join("Mei/README.md"),
            mode: 0o644,
        },
        Entry {
            name: "LICENSE".to_owned(),
            source: root.join("LICENSE"),
            mode: 0o644,
        },
    ];
    assemble(root, &stem, &entries)
}

fn assemble(root: &Path, stem: &str, entries: &[Entry]) -> Result<String, XtaskError> {
    let out_dir = root.join("target").join("package");
    std::fs::create_dir_all(&out_dir).map_err(|source| XtaskError::Io {
        path: out_dir.display().to_string(),
        source,
    })?;
    let archive = out_dir.join(format!("{stem}.zip"));
    write_archive(&archive, stem, entries)?;
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

fn write_archive(archive: &Path, stem: &str, entries: &[Entry]) -> Result<(), XtaskError> {
    let file = std::fs::File::create(archive).map_err(|source| XtaskError::Io {
        path: archive.display().to_string(),
        source,
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let io = |path: &Path, source: std::io::Error| XtaskError::Io {
        path: path.display().to_string(),
        source,
    };
    for entry in entries {
        let bytes = std::fs::read(&entry.source).map_err(|err| io(&entry.source, err))?;
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(9))
            .last_modified_time(zip::DateTime::default())
            // zip 9 marks an entry made on Windows as DOS unless the system
            // is named, and a DOS entry's unix mode is read back as the DOS
            // interpretation, so the executable bits a person needs after
            // unpacking would be lost. The archives are built on three
            // hosts and unpacked on others, so the entry says Unix
            // wherever it was written.
            .system(zip::System::Unix)
            .unix_permissions(entry.mode);
        zip.start_file(format!("{stem}/{}", entry.name), options)
            .map_err(|err| XtaskError::Cmd {
                cmd: "package".to_owned(),
                msg: format!("{}: {err}", entry.name),
            })?;
        zip.write_all(&bytes).map_err(|err| io(archive, err))?;
    }
    zip.finish().map_err(|err| XtaskError::Cmd {
        cmd: "package".to_owned(),
        msg: err.to_string(),
    })?;
    Ok(())
}

pub(crate) fn workspace_package(root: &Path, key: &str) -> Result<String, XtaskError> {
    let path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: err.to_string(),
    })?;
    package_field(&parsed, key)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: path.display().to_string(),
            msg: format!("workspace.package.{key} is missing"),
        })
}

pub(crate) fn package_field<'a>(manifest: &'a toml::Value, key: &str) -> Option<&'a str> {
    manifest
        .get("workspace")
        .and_then(|it| it.get("package"))
        .and_then(|it| it.get(key))
        .and_then(toml::Value::as_str)
}

#[cfg(test)]
#[path = "package_tests.rs"]
mod tests;
