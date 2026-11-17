// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use sha2::{Digest as _, Sha256};
use std::io::Read as _;

use std::path::{Path, PathBuf};

use crate::platform::{
    MEI_ARCHIVE_PREFIX, MEI_ROOT_PACKAGE, PLATFORMS, Platform, ROOT_PACKAGE, calyx_binary,
    mei_binary, mei_package,
};
use crate::report::XtaskError;

mod system;

const SHIM: &str = include_str!("channel/shim.js");
const RUNTIME_ENTRY: &str = "sprawling-runtime";
const LAUNCHER: &str = include_str!("channel/launcher.cmd");

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
        let is_wanted = {
            let name = entry.name().map_err(|err| XtaskError::Cmd {
                cmd: format!("read {}", archive.display()),
                msg: err.to_string(),
            })?;
            name.rsplit('/').next() == Some(wanted)
        };
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

struct Stem<'a> {
    version: &'a str,
    repository: &'a str,
}

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

struct BinaryPackage<'a> {
    platform: &'a Platform,
    name: &'a str,
    binaries: &'a [&'a str],
    description: String,
}

fn carry(
    stem: &Stem<'_>,
    archive: &Path,
    package: &BinaryPackage<'_>,
    out: &Path,
) -> Result<(), XtaskError> {
    let dir = out.join(package.name);
    for name in package.binaries {
        let binary = extract(archive, name)?;
        if binary.mode & 0o111 == 0 {
            return Err(XtaskError::Cmd {
                cmd: "npm channel".to_owned(),
                msg: format!("{} has no executable permission bits", archive.display()),
            });
        }
        let path = dir.join("bin").join(name);
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
    }
    let extra = format!(
        "  \"os\": [\"{}\"],\n  \"cpu\": [\"{}\"],\n  \"files\": [\"bin\"]\n",
        package.platform.os, package.platform.cpu
    );
    write(
        &dir.join("package.json"),
        manifest(stem, package.name, &package.description, &extra).as_bytes(),
    )
}

fn mei_description(root: &Path) -> Result<String, XtaskError> {
    let path = root.join("Mei/Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: path.display().to_string(),
        msg: err.to_string(),
    })?;
    parsed
        .get("package")
        .and_then(|package| package.get("description"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: path.display().to_string(),
            msg: "package.description is missing".to_owned(),
        })
}

fn write_mei_channel(
    root: &Path,
    stem: &Stem<'_>,
    archives: &[PathBuf],
    out: &Path,
) -> Result<Vec<String>, XtaskError> {
    let description = mei_description(root)?;
    let mut carried: Vec<(&Platform, &'static str)> = Vec::new();
    for archive in archives {
        let name = archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| XtaskError::Cmd {
                cmd: "npm channel".to_owned(),
                msg: format!("{} is not an archive file path", archive.display()),
            })?;
        if !name.starts_with(MEI_ARCHIVE_PREFIX) {
            continue;
        }
        let row = PLATFORMS.iter().find(|row| name.ends_with(row.suffix));
        let Some(row) = row else {
            return Err(XtaskError::Cmd {
                cmd: "npm channel".to_owned(),
                msg: format!("{name} matches no platform this channel knows"),
            });
        };
        let Some(package) = mei_package(row) else {
            return Err(XtaskError::Cmd {
                cmd: "npm channel".to_owned(),
                msg: format!(
                    "{name} is not a supported Mei archive; Mei publishes npm binaries \
                     for Windows x64, macOS arm64, and Linux x64 musl"
                ),
            });
        };
        let binaries = [mei_binary(row), calyx_binary(row)];
        carry(
            stem,
            archive,
            &BinaryPackage {
                platform: row,
                name: package,
                binaries: &binaries,
                description: format!(
                    "Mei headless core and Calyx interactive face for {} {}.",
                    row.os, row.cpu
                ),
            },
            out,
        )?;
        carried.push((row, package));
    }
    if carried.is_empty() {
        return Err(XtaskError::Cmd {
            cmd: "npm channel".to_owned(),
            msg: "this tag holds no Mei release archive".to_owned(),
        });
    }

    let optional = carried
        .iter()
        .map(|(_, package)| format!("    \"{package}\": \"{version}\"", version = stem.version))
        .collect::<Vec<_>>()
        .join(",\n");
    let extra = format!(
        "  \"bin\": {{ \"mei\": \"bin/mei.js\" }},\n  \
         \"files\": [\"bin\"],\n  \"optionalDependencies\": {{\n{optional}\n  }}\n"
    );
    let dir = out.join(MEI_ROOT_PACKAGE);
    write(
        &dir.join("package.json"),
        manifest(stem, MEI_ROOT_PACKAGE, &description, &extra).as_bytes(),
    )?;
    write(
        &dir.join("bin/mei.js"),
        format!("#!/usr/bin/env bun\n{SHIM}").as_bytes(),
    )?;
    Ok(carried
        .into_iter()
        .map(|(_, package)| package.to_owned())
        .collect())
}

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
        if name.starts_with(MEI_ARCHIVE_PREFIX) {
            continue;
        }
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
        carry(
            &stem,
            archive,
            &BinaryPackage {
                platform: row,
                name: row.package,
                binaries: &[row.binary],
                description: format!("The sprawling binary for {} {}.", row.os, row.cpu),
            },
            out,
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
    let mei_names = write_mei_channel(root, &stem, &archives, out)?.join(", ");
    Ok(format!(
        "npm channel {version} written to {}: sprawling + {names}; \
         {MEI_ROOT_PACKAGE} + {mei_names}",
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
