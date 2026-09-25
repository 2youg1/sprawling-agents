// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's git history in a bundle: one pack of every object the refs
//! reach, and the refs themselves (memory-SPEC.md 8-12).
//!
//! Two files and nothing else travel. A repository's `hooks/` and
//! `config` are what a forged bundle would plant, so the restoring side
//! takes both from a fresh `init` and never from the bundle.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::alias::WriteTarget;
use crate::error::{MemoryError, io_err};
use crate::vfs::Vfs;

use super::files::write_file;

/// The bundle's directory for the history.
pub(crate) const HISTORY: &str = "history";
const PACK: &str = "history.pack";
const REFS: &str = "refs";
const SYMBOLIC: &str = "ref:";
const REFLOG: &str = "restore a city bundle";

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> MemoryError {
    move |err| MemoryError::Bundle {
        op,
        detail: err.message().to_owned(),
    }
}

/// Packs the history of the repository at `city_root` into `dest`.
/// A city that is no repository has no history, and its bundle carries
/// no history directory.
///
/// # Errors
/// `MemoryError::Bundle` for a repository that cannot be read, git2's
/// own refusal of a ref name that is not UTF-8 among them; I/O failures
/// naming the path.
pub(crate) fn export(vfs: &mut dyn Vfs, city_root: &Path, dest: &Path) -> Result<(), MemoryError> {
    let repo = match git2::Repository::open(city_root) {
        Ok(repo) => repo,
        Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),
        Err(err) => return Err(git_err("export")(err)),
    };
    let mut pack = repo.packbuilder().map_err(git_err("export"))?;
    let mut walk = repo.revwalk().map_err(git_err("export"))?;
    let mut refs = String::new();
    let head = repo.find_reference("HEAD").map_err(git_err("export"))?;
    let listed = repo.references().map_err(git_err("export"))?;
    for reference in std::iter::once(Ok(head)).chain(listed) {
        let reference = reference.map_err(git_err("export"))?;
        let name = reference.name().map_err(git_err("export"))?;
        if let Some(target) = reference.symbolic_target().map_err(git_err("export"))? {
            refs.push_str(&format!("{SYMBOLIC}{target} {name}\n"));
        } else if let Some(oid) = reference.target() {
            pack.insert_recursive(oid, None)
                .map_err(git_err("export"))?;
            if let Ok(commit) = reference.peel_to_commit() {
                walk.push(commit.id()).map_err(git_err("export"))?;
            }
            refs.push_str(&format!("{oid} {name}\n"));
        }
    }
    pack.insert_walk(&mut walk).map_err(git_err("export"))?;
    let dir = dest.join(HISTORY);
    vfs.create_dir_all(&dir)
        .map_err(io_err("make a bundle directory", &dir))?;
    if pack.object_count() > 0 {
        let mut bytes = git2::Buf::new();
        pack.write_buf(&mut bytes).map_err(git_err("export"))?;
        let target = WriteTarget::at("write a history pack", &dir.join(PACK))?;
        write_file(vfs, &target, &bytes)?;
    }
    let target = WriteTarget::at("write the history refs", &dir.join(REFS))?;
    write_file(vfs, &target, refs.as_bytes())
}

/// One ref as the bundle states it.
enum Target {
    Object(git2::Oid),
    Symbolic(String),
}

/// A bundle's history, read and checked before anything is restored.
pub(crate) struct History {
    pack: Option<PathBuf>,
    refs: Vec<(String, Target)>,
}

impl History {
    /// Reads the history a bundle carries, refusing it whole when a ref
    /// is malformed or `city_root` already holds a repository.
    ///
    /// # Errors
    /// `MemoryError::Bundle` naming the first malformed line or the
    /// occupied repository; I/O failures naming the path.
    pub(crate) fn read(
        vfs: &dyn Vfs,
        bundle: &Path,
        city_root: &Path,
    ) -> Result<Option<History>, MemoryError> {
        let dir = bundle.join(HISTORY);
        let at = dir.join(REFS);
        if !vfs.exists(&at) {
            return Ok(None);
        }
        let occupied = city_root.join(kernel::GIT_METADATA);
        // A link or a gitfile named `.git` occupies the name as surely
        // as a directory does, so the name is asked about, not followed.
        if occupied.symlink_metadata().is_ok() {
            return Err(MemoryError::Bundle {
                op: "restore",
                detail: format!("{} already holds a repository", occupied.display()),
            });
        }
        let bytes = vfs
            .read(&at)
            .map_err(io_err("read the history refs", &at))?;
        let text = String::from_utf8(bytes).map_err(|_| malformed(&at))?;
        let refs = text
            .lines()
            .map(|line| parse(line).ok_or_else(|| malformed(&at)))
            .collect::<Result<Vec<_>, _>>()?;
        let pack = dir.join(PACK);
        Ok(Some(History {
            pack: vfs.exists(&pack).then_some(pack),
            refs,
        }))
    }

    /// Initialises a repository at `city_root`, indexes the pack into
    /// it, sets every ref, and reads `HEAD`'s tree into the index, so
    /// `git status` reports only what was uncommitted at export.
    ///
    /// # Errors
    /// `MemoryError::Bundle` for any git failure, including a pack that
    /// does not index; I/O failures naming the path.
    pub(crate) fn land(self, vfs: &dyn Vfs, city_root: &Path) -> Result<(), MemoryError> {
        let repo = git2::Repository::init(city_root).map_err(git_err("restore"))?;
        if let Some(at) = self.pack {
            let bytes = vfs.read(&at).map_err(io_err("read a history pack", &at))?;
            let odb = repo.odb().map_err(git_err("restore"))?;
            let mut writer = odb.packwriter().map_err(git_err("restore"))?;
            writer
                .write_all(&bytes)
                .map_err(io_err("index a history pack", &at))?;
            writer.commit().map_err(git_err("restore"))?;
        }
        for (name, target) in &self.refs {
            match target {
                Target::Object(oid) => repo.reference(name, *oid, true, REFLOG),
                Target::Symbolic(to) => repo.reference_symbolic(name, to, true, REFLOG),
            }
            .map_err(git_err("restore"))?;
        }
        match repo.head().and_then(|head| head.peel_to_tree()) {
            Ok(tree) => {
                let mut index = repo.index().map_err(git_err("restore"))?;
                index.read_tree(&tree).map_err(git_err("restore"))?;
                index.write().map_err(git_err("restore"))
            }
            Err(err) if err.code() == git2::ErrorCode::UnbornBranch => Ok(()),
            Err(err) => Err(git_err("restore")(err)),
        }
    }
}

/// One line of the refs file; `None` for anything a ref cannot be.
/// Only `HEAD` and names under `refs/` are refs a restore may write.
fn parse(line: &str) -> Option<(String, Target)> {
    let (target, name) = line.split_once(' ')?;
    let admitted = |ref_name: &str| ref_name == "HEAD" || ref_name.starts_with("refs/");
    let valid = |ref_name: &str| admitted(ref_name) && git2::Reference::is_valid_name(ref_name);
    if !valid(name) {
        return None;
    }
    let target = match target.strip_prefix(SYMBOLIC) {
        Some(to) if valid(to) => Target::Symbolic(to.to_owned()),
        Some(_) => return None,
        None => Target::Object(git2::Oid::from_str(target).ok()?),
    };
    Some((name.to_owned(), target))
}

fn malformed(at: &Path) -> MemoryError {
    MemoryError::Bundle {
        op: "restore",
        detail: format!("{} holds a line that is not a ref", at.display()),
    }
}
