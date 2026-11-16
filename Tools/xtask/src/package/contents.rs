// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use crate::report::XtaskError;
use crate::sbom;
use crate::walk;

pub(super) enum Packaged {
    Binary,
    Document {
        name: &'static str,
        source: &'static str,
    },
    Sbom,
    Skills,
}

pub(super) const ARCHIVE: [Packaged; 5] = [
    Packaged::Binary,
    Packaged::Document {
        name: "QUICKSTART.md",
        source: "docs/dist/QUICKSTART.md",
    },
    Packaged::Document {
        name: "LICENSE",
        source: "LICENSE",
    },
    Packaged::Skills,
    Packaged::Sbom,
];

const EXECUTABLE_MODE: u32 = 0o755;

const READABLE_MODE: u32 = 0o644;

const SKILLS_DIR: &str = "Sprawling/city/skills";

const SKILLS_ENTRY: &str = "skills";

pub(super) struct Executable<'b> {
    pub(super) name: &'static str,
    pub(super) path: &'b Path,
}

#[derive(Debug)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) source: PathBuf,
    pub(super) mode: u32,
}

pub(super) fn entries(root: &Path, binary: &Executable) -> Result<Vec<Entry>, XtaskError> {
    let mut out = Vec::new();
    for part in &ARCHIVE {
        out.extend(part.resolve(root, binary)?);
    }
    Ok(out)
}

impl Packaged {
    fn resolve(&self, root: &Path, binary: &Executable) -> Result<Vec<Entry>, XtaskError> {
        let entries = match *self {
            Self::Binary => vec![Entry {
                name: binary.name.to_owned(),
                source: binary.path.to_path_buf(),
                mode: EXECUTABLE_MODE,
            }],
            Self::Document { name, source } => vec![Entry {
                name: name.to_owned(),
                source: root.join(source),
                mode: READABLE_MODE,
            }],
            Self::Sbom => vec![Entry {
                name: sbom_name()?,
                source: root.join(sbom::SBOM),
                mode: READABLE_MODE,
            }],
            Self::Skills => return skills(root),
        };
        for entry in &entries {
            if !entry.source.is_file() {
                return Err(XtaskError::Cmd {
                    cmd: "package".to_owned(),
                    msg: format!(
                        "the archive carries {} and {} is not there; {}",
                        entry.name,
                        entry.source.display(),
                        self.recovery()
                    ),
                });
            }
        }
        Ok(entries)
    }

    fn recovery(&self) -> &'static str {
        match *self {
            Self::Binary => "run `just dist` first",
            Self::Document { .. } => {
                "restore the document, or change this table and the \
                                      archive in one change-set"
            }
            Self::Sbom => "run `just sbom`, or `just dist`, which ends with it",
            Self::Skills => {
                "restore `Sprawling/city/skills/` from the tree, or change this table and the archive in \
                 one change-set"
            }
        }
    }
}

fn skills(root: &Path) -> Result<Vec<Entry>, XtaskError> {
    let dir = root.join(SKILLS_DIR);
    if !dir.is_dir() {
        return Err(XtaskError::Cmd {
            cmd: "package".to_owned(),
            msg: format!(
                "the archive carries the skills and {SKILLS_DIR}/ is not there; restore the \
                 directory from the tree, or change this table and the archive in one \
                 change-set"
            ),
        });
    }
    let mut out = Vec::new();
    for file in walk::files(&dir)? {
        out.push(Entry {
            name: format!("{SKILLS_ENTRY}/{}", walk::rel(&dir, &file)),
            source: file,
            mode: READABLE_MODE,
        });
    }
    Ok(out)
}

fn sbom_name() -> Result<String, XtaskError> {
    Path::new(sbom::SBOM)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: "Tools/xtask/src/sbom.rs".to_owned(),
            msg: format!("`{}` names no file to put in the archive", sbom::SBOM),
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
    use super::{ARCHIVE, Executable, Packaged, SKILLS_DIR, entries, skills};
    use crate::sbom::SBOM;

    fn repo_root() -> &'static std::path::Path {
        crate::root::this_checkout()
    }

    #[test]
    fn the_skills_ride_in_the_archive_under_their_own_paths() {
        let dir = std::env::temp_dir().join(format!("sprawling-skills-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let skill = dir.join(SKILLS_DIR).join("alpha");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), b"one").unwrap();
        std::fs::write(dir.join(SKILLS_DIR).join("LICENSES.md"), b"two").unwrap();

        let found = skills(&dir).unwrap();
        let names: Vec<&str> = found.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["skills/LICENSES.md", "skills/alpha/SKILL.md"]);
        assert!(found.iter().all(|entry| entry.mode == super::READABLE_MODE));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_skills_directory_refuses_the_archive() {
        let dir = std::env::temp_dir().join(format!("sprawling-noskills-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();

        let refusal = skills(&dir).expect_err("a tree with no skills must not assemble");
        let said = refusal.to_string();
        assert!(said.contains("skills"), "{said}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn every_packaged_document_is_in_the_tree() {
        for part in &ARCHIVE {
            let Packaged::Document { name, source } = *part else {
                continue;
            };
            assert!(
                repo_root().join(source).is_file(),
                "the archive packages {name} from {source}, which is not in this tree"
            );
        }
    }

    fn assembled(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("sprawling-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        for part in &ARCHIVE {
            let at = match *part {
                Packaged::Binary | Packaged::Skills => continue,
                Packaged::Document { source, .. } => root.join(source),
                Packaged::Sbom => root.join(SBOM),
            };
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::fs::write(&at, b"content").unwrap();
        }
        let skill = root.join(SKILLS_DIR).join("example");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), b"skill").unwrap();
        let binary = root.join("target").join("release").join("sprawling");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"binary").unwrap();
        root
    }

    #[test]
    fn an_absent_bill_of_materials_refuses_the_archive() {
        let root = assembled("sbom");
        std::fs::remove_file(root.join(SBOM)).unwrap();
        let binary = root.join("target").join("release").join("sprawling");

        let refusal = entries(
            &root,
            &Executable {
                name: "sprawling",
                path: &binary,
            },
        )
        .expect_err("a tree with no bill of materials must not assemble an archive");
        let said = refusal.to_string();
        assert!(said.contains("sbom.cdx.json"), "{said}");
        assert!(said.contains("just sbom"), "{said}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn every_part_is_packed_and_only_the_binary_is_runnable() {
        let root = assembled("archive");
        let binary = root.join("target").join("release").join("sprawling");
        let found = entries(
            &root,
            &Executable {
                name: "sprawling",
                path: &binary,
            },
        )
        .unwrap();

        let names: Vec<&str> = found.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "sprawling",
                "QUICKSTART.md",
                "LICENSE",
                "skills/example/SKILL.md",
                "sbom.cdx.json"
            ]
        );
        let runnable: Vec<&str> = found
            .iter()
            .filter(|entry| entry.mode == 0o755)
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(runnable, ["sprawling"]);
        std::fs::remove_dir_all(&root).ok();
    }
}
