// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What rides in a release archive, and where each part is read from.
//!
//! **The list is closed, and every part of it is required.** Whether the
//! archive carried the bill of materials used to depend on whether
//! `target/` happened to hold one: an archive assembled in a tree where
//! `just sbom` had not run was one entry short and said nothing about
//! it, so two people packaging one tag could ship archives that disagree
//! about what is inside them. A part that cannot be found is now a
//! refusal naming the recipe that produces it.

use std::path::{Path, PathBuf};

use crate::report::XtaskError;
use crate::sbom;
use crate::walk;

/// One part of a release archive.
///
/// Exhaustive on purpose: the two questions the packager asks of each
/// part — what a person finds after unpacking, and what has to exist for
/// it to be there — have one answer each per variant.
pub(super) enum Packaged {
    /// The executable, under the name its target gives it.
    Binary,
    /// A document a person reads beside the binary, and where the tree
    /// keeps it. `QUICKSTART.md` moved into `docs/` once while this, its
    /// only reader, kept asking for the old path — so every archive
    /// after that move failed to assemble, on a release runner, which is
    /// the one place nobody watches until a tag is already cut.
    Document {
        name: &'static str,
        source: &'static str,
    },
    /// The CycloneDX bill of materials, so a person who wants to know
    /// what is inside the binary does not have to build it.
    Sbom,
    /// The skills a release ships: `skills/`, walked as it stands. One
    /// part rather than one row per skill, because the list of skills is
    /// the directory's business and a closed per-file list here would be
    /// a second home for it.
    Skills,
}

/// Everything a release archive carries, in the order it is written.
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

/// Executable bits. The archive is the only thing that carries them to a
/// machine that has never seen this file, and a binary that arrives
/// without them is a binary nobody can run.
const EXECUTABLE_MODE: u32 = 0o755;

/// Everything else is read, not run.
const READABLE_MODE: u32 = 0o644;

/// The name of the tree of skills, spelled once for the walk and for
/// every entry name the walk produces.
const SKILLS_DIR: &str = "skills";

/// The release build this archive is assembled around: where it landed,
/// and what the target it was built for calls it.
pub(super) struct Executable<'b> {
    pub(super) name: &'static str,
    pub(super) path: &'b Path,
}

/// One entry of the archive, resolved to bytes on disk.
#[derive(Debug)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) source: PathBuf,
    pub(super) mode: u32,
}

/// Every entry of the archive, or the refusal that names the missing one.
///
/// # Errors
/// When a part of the archive is not on disk, with the recipe that
/// writes it.
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

    /// What a person does about this part being absent.
    fn recovery(&self) -> &'static str {
        match *self {
            Self::Binary => "run `just dist` first",
            Self::Document { .. } => {
                "restore the document, or change this table and the \
                                      archive in one change-set"
            }
            Self::Sbom => "run `just sbom`, or `just dist`, which ends with it",
            Self::Skills => {
                "restore `skills/` from the tree, or change this table and the archive in \
                 one change-set"
            }
        }
    }
}

/// Every file under `skills/`, one entry each, named `skills/<path>`.
///
/// Walked rather than listed: which skills a release carries is the
/// directory's business, and `walk::files` already returns them sorted
/// by relative path, so two packagings of one tree produce one archive.
/// `skills/LICENSES.md` rides the same walk, which is how the licence
/// text travels with the files it covers.
///
/// # Errors
/// Refuses a tree with no `skills/` directory - the part is required
/// like every other, and an archive that silently ships without the
/// skills is the defect this table exists to prevent.
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
            name: format!("{SKILLS_DIR}/{}", walk::rel(&dir, &file)),
            source: file,
            mode: READABLE_MODE,
        });
    }
    Ok(out)
}

/// The name the archive carries the bill of materials under: the last
/// segment of the one path that says where it is written.
fn sbom_name() -> Result<String, XtaskError> {
    Path::new(sbom::SBOM)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_owned)
        .ok_or_else(|| XtaskError::Doc {
            file: "xtask/src/sbom.rs".to_owned(),
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
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask sits one level under the repository root")
    }

    /// The skills ride in the archive exactly as they stand on disk: the
    /// list of skills is the directory's business, and a person who
    /// unpacks a release finds them where the harnesses look.
    #[test]
    fn the_skills_ride_in_the_archive_under_their_own_paths() {
        let dir = std::env::temp_dir().join(format!("sprawling-skills-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let skill = dir.join("skills").join("alpha");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), b"one").unwrap();
        std::fs::write(dir.join("skills").join("LICENSES.md"), b"two").unwrap();

        let found = skills(&dir).unwrap();
        let names: Vec<&str> = found.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["skills/LICENSES.md", "skills/alpha/SKILL.md"]);
        assert!(found.iter().all(|entry| entry.mode == super::READABLE_MODE));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A part of the closed list that is not on disk refuses the archive
    /// and names the way out - the same contract every other part has.
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

    /// A document the archive is assembled from has to be in the tree the
    /// archive is assembled out of. This fails at `cargo nextest`, on
    /// every push, rather than on a release runner after a tag is cut.
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

    /// A tree holding every part of the archive, and the binary to
    /// assemble it around.
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
        std::fs::write(root.join("sprawling"), b"binary").unwrap();
        root
    }

    /// The defect this table exists for: an absent bill of materials used
    /// to leave the archive one entry short and silent about it.
    #[test]
    fn an_absent_bill_of_materials_refuses_the_archive() {
        let root = assembled("sbom");
        std::fs::remove_file(root.join(SBOM)).unwrap();
        let binary = root.join("sprawling");

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

    /// The executable is the one entry that arrives runnable, and the
    /// archive carries every part of the table.
    #[test]
    fn every_part_is_packed_and_only_the_binary_is_runnable() {
        let root = assembled("archive");
        let binary = root.join("sprawling");
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
