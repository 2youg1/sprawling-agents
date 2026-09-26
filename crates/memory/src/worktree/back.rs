// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Going back to a point, and taking one file back from a point.
//!
//! `adversary/design/GoingBack.lean` holds which properties these two
//! must keep (memory-SPEC.md 8-27): a tree opened at a point holds that
//! point's files, a live name or an existing line of work is refused
//! rather than replaced, and neither step touches the trunk or another
//! run's tree.

use std::path::{Component, Path};

use kernel::GitOid;
use kernel::event::record::FileRestored;

use crate::alias::WriteTarget;
use crate::bundle::landing::{Bits, land};
use crate::error::MemoryError;
use crate::real_fs::RealFs;

use super::lease::WorktreeLease;
use super::name::WorktreeName;
use super::trees::Worktrees;

impl Worktrees {
    /// Opens a new tree whose branch starts at `point`. The trunk and the
    /// city working tree do not move: another run may be writing, and
    /// the trunk is where its work lands.
    ///
    /// # Errors
    /// `WorktreeBusy` when the name holds a live tree or a branch, or the
    /// city working tree is over the ceiling; `Worktree` when `point` is
    /// not a commit of the city.
    pub fn claim_at(
        &self,
        name: &WorktreeName,
        point: &GitOid,
    ) -> Result<WorktreeLease, MemoryError> {
        let busy = |detail: &str| MemoryError::WorktreeBusy {
            name: name.as_str().to_owned(),
            detail: detail.to_owned(),
        };
        if self.live()?.contains(name) {
            return Err(busy("a live tree holds this name"));
        }
        self.refuse_oversized(name)?;
        let commit = self.commit_at(point)?;
        // Never forced: an existing branch is a line of work, and
        // replacing it would drop somebody's writes.
        let branch = self
            .repo
            .branch(name.as_str(), &commit, false)
            .map_err(|err| {
                if err.code() == git2::ErrorCode::Exists {
                    busy("a line of work already has this name")
                } else {
                    MemoryError::Worktree {
                        op: "branch at the point to go back to",
                        detail: format!("{point}: {err}"),
                    }
                }
            })?;
        match self.add_tree(name, Some(branch.get())) {
            Ok(lease) => Ok(lease),
            Err(refusal) => {
                // The branch was made for this tree alone; left behind, it
                // would hold the name as a line of work nobody started.
                let mut branch = branch;
                branch.delete().map_err(|err| MemoryError::Worktree {
                    op: "remove the branch of a tree that did not open",
                    detail: format!("{}: {err}; the tree failed with: {refusal}", name.as_str()),
                })?;
                Err(refusal)
            }
        }
    }

    /// Takes `path` back from `point` into the tree `lease` holds: the
    /// point's bytes when it holds the file, and no file when it does
    /// not. No other tree and not the trunk are touched. Returns the
    /// `file_restored` record of this step for the caller to append.
    ///
    /// # Errors
    /// Refuses a path that is empty, absolute, climbs, or names the
    /// reserved subtree; a point that is not a commit of the city; and a
    /// path the point holds as something other than a file.
    pub fn restore_file(
        &self,
        lease: &WorktreeLease,
        point: &GitOid,
        path: &Path,
    ) -> Result<FileRestored, MemoryError> {
        let refuse = |detail: String| MemoryError::Worktree {
            op: "restore a file from a point",
            detail,
        };
        let inside = in_tree(path).ok_or_else(|| {
            refuse(format!(
                "{} is not a plain path inside the tree",
                path.display()
            ))
        })?;
        let tree = self
            .commit_at(point)?
            .tree()
            .map_err(|err| refuse(format!("{point}: {err}")))?;
        let target = WriteTarget::at("restore a file from a point", &lease.path().join(path))?;
        let io = |source: std::io::Error| MemoryError::Io {
            op: "restore a file from a point",
            path: target.as_path().to_path_buf(),
            source,
        };
        let restored = FileRestored {
            name: lease.name().as_str().to_owned(),
            path: inside,
            point: *point,
        };
        let entry = match tree.get_path(Path::new(&restored.path)) {
            Ok(entry) => entry,
            Err(err) if err.code() == git2::ErrorCode::NotFound => {
                return match std::fs::remove_file(target.as_path()) {
                    Ok(()) => Ok(restored),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(restored),
                    Err(err) => Err(io(err)),
                };
            }
            Err(err) => return Err(refuse(format!("{}: {err}", path.display()))),
        };
        let object = entry
            .to_object(&self.repo)
            .map_err(|err| refuse(format!("{}: {err}", path.display())))?;
        let blob = object
            .as_blob()
            .ok_or_else(|| refuse(format!("{} is not a file at {point}", path.display())))?;
        if let Some(parent) = target.as_path().parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        land(&mut RealFs::new(), target, blob.content(), Bits::OfReplaced)?;
        Ok(restored)
    }

    fn commit_at(&self, point: &GitOid) -> Result<git2::Commit<'_>, MemoryError> {
        let missing = |err: git2::Error| MemoryError::Worktree {
            op: "find the point to go back to",
            detail: format!("{point}: {err}"),
        };
        let oid = git2::Oid::from_str(&point.to_string()).map_err(missing)?;
        self.repo.find_commit(oid).map_err(missing)
    }
}

/// `path` in the spelling a git tree uses, or `None` when it is empty,
/// leaves the tree, or names the reserved subtree.
fn in_tree(path: &Path) -> Option<String> {
    let segments = path
        .components()
        .map(|component| match component {
            Component::Normal(segment) => segment.to_str(),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!segments.is_empty() && crate::reserved::outside_reserved(path)).then(|| segments.join("/"))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
