// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint fences: base, wave pre/post.

use std::path::Path;

use kernel::event::record::{CheckpointCommitted, Commit};
use kernel::{Address, GitOid, Payload, TimeMs};
use serde_json::{Map, Value};

use crate::error::MemoryError;

use super::provenance::Provenance;
use super::scan::CommitPlan;

/// How a fence names itself in a commit subject. The whole set, because
/// a fence that showed one of several prefixes would read like a fence
/// that staged one of them.
fn subject_of(scopes: &[String]) -> String {
    if scopes.is_empty() {
        return "checkpoint: .".to_owned();
    }
    format!("checkpoint: {}", scopes.join(" "))
}

/// Where a wave fence is filed: under `refs/sprawling/`, which no
/// branch listing, push or `git log` walks by accident, and named by the
/// commit itself. A run fences through more than one handle - the lane's
/// own and the bench's forecast net - and the reference is the only thing
/// that keeps a fence from `git gc`, so its name cannot come from a count
/// that a second handle also keeps.
fn fence_ref(of: &Provenance, oid: git2::Oid) -> String {
    format!("refs/sprawling/runs/{}/{oid}", of.run())
}

/// The `checkpoint_committed` payload, in one place so a fence and a
/// base commit cannot describe themselves differently.
///
/// It carries what the commit's own trailers carry that the record
/// cannot say for itself: the model and the effort. The run and the
/// actor are the record's identity and are not repeated here.
fn committed(
    oid: git2::Oid,
    of: &Provenance,
    scopes: &[String],
    files: Vec<String>,
) -> Result<Payload, MemoryError> {
    let spelled = oid.to_string();
    let oid = GitOid::parse(&spelled).ok_or_else(|| MemoryError::Checkpoint {
        op: "record a checkpoint commit",
        detail: format!("git named this commit {spelled}, which is not 40 hex digits"),
    })?;
    let record = CheckpointCommitted::Committed(Commit {
        oid,
        by: of.attribution(),
        scope: scopes.to_vec(),
        files,
    });
    Payload::of(&record).map_err(|source| MemoryError::Draft { source })
}

pub struct Checkpoint {
    pub(crate) repo: git2::Repository,
    /// The last commit this handle made, fence or landing. The scan
    /// compares against it, because a fence no longer moves HEAD.
    pub(crate) last: Option<git2::Oid>,
}

pub(crate) fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> MemoryError {
    move |err| MemoryError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

impl Checkpoint {
    /// Opens the city repository, initialising one when absent. The
    /// genesis commit is the first wave's, not this call's: an empty
    /// repository is a valid state, and inventing history here would
    /// make the first checkpoint unattributable.
    pub fn open(city_root: &Path) -> Result<Checkpoint, MemoryError> {
        let repo = match git2::Repository::open(city_root) {
            Ok(repo) => repo,
            Err(_) => git2::Repository::init(city_root).map_err(git_err("init repository"))?,
        };
        // The city's files round-trip byte for byte, whatever this
        // machine's git is configured to do to other people's
        // repositories. A checkout that rewrote line endings would make
        // a file disagree with the hash the ledger holds for it, and the
        // disagreement would look like corruption rather than like a
        // setting. Set on every open, because the setting is a property
        // of this repository rather than of the moment it was created.
        repo.config()
            .and_then(|mut config| config.set_bool("core.autocrlf", false))
            .map_err(git_err("pin the repository's line endings"))?;
        Ok(Checkpoint { repo, last: None })
    }

    /// Makes sure the city has one commit, and makes no more than that.
    ///
    /// A worktree branches from a commit, so a city that has never been
    /// fenced cannot lend a tree. Committing on every dispatch would
    /// move the trunk under every request already waiting. Returns the
    /// commit it made, or `None` when there already was one.
    ///
    /// # Errors
    /// Propagates whatever staging and committing report.
    pub fn ensure_base(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Option<Payload>, MemoryError> {
        if self.repo.head().is_ok() {
            return Ok(None);
        }
        let files = self.stage_scopes(scopes)?;
        self.scan_staged()?;
        // The one fence that moves the branch: a worktree branches from a
        // commit, and a city that has none can lend no tree.
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            onto_head: true,
        })?;
        committed(oid, of, scopes, files).map(Some)
    }

    /// The pre-wave fence: stage everything under `scopes`, scan it, and
    /// commit at the injected time. Returns the `checkpoint_committed`
    /// payload.
    ///
    /// **`scopes` is the run's write domain, not its room** (memory-SPEC
    /// section 8-18). The two were allowed to differ once, and every
    /// file a resident wrote between them - a building's own documents,
    /// a second declared prefix - was staged by no fence, reported by no
    /// `changes` query, and restorable from no `file_discarded` record.
    ///
    /// **The branch does not move**: the commit is written
    /// with no reference update and pointed at by
    /// `refs/sprawling/runs/<run>/<oid>`, so a person whose own folder
    /// became this city keeps their own history instead of one
    /// `checkpoint:` line per tool wave. The oid means what it always
    /// meant - the tree is there and `wave_post` restores from it.
    ///
    /// # Errors
    /// Propagates staging, the staged-secret scan, the commit, and a
    /// reference the repository will not write.
    pub fn wave_pre(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Payload, MemoryError> {
        let files = self.stage_scopes(scopes)?;
        self.scan_staged()?;
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            onto_head: false,
        })?;
        self.repo
            .reference(&fence_ref(of, oid), oid, true, "sprawling: a wave fence")
            .map_err(git_err("file a wave fence"))?;
        committed(oid, of, scopes, files)
    }

    /// The post-wave sweep: every path present at `pre_oid` and gone
    /// from the working tree becomes a `file_discarded` payload whose
    /// restoration points back into that commit.
    ///
    /// **The question is existence, and only existence.** The tree is
    /// walked once and each blob is looked for on disk; a path that is
    /// not there is a deletion. Asking git for the tree-to-workdir diff
    /// instead would hash every path the tree and the worktree agree on,
    /// and the city itself keeps writing some of those paths after the
    /// fence (a room's session projection, a worktree under a lease); a
    /// hash taken through a Windows directory entry that trails the open
    /// handle refuses the whole sweep. A sweep needs `Deleted` deltas,
    /// and a deletion is answered without reading any content.
    ///
    /// **The tree is the fence's own write domain**, not the whole city:
    /// `wave_pre` walked exactly these paths before the wave began, so
    /// the walk costs what staging already cost.
    ///
    /// # Errors
    /// Propagates a `pre_oid` that is not an object, a commit whose tree
    /// cannot be read, and a working-tree path that cannot be examined.
    pub fn wave_post(&mut self, pre_oid: &str) -> Result<Vec<Payload>, MemoryError> {
        let oid = git2::Oid::from_str(pre_oid).map_err(git_err("parse checkpoint oid"))?;
        let commit = self
            .repo
            .find_commit(oid)
            .map_err(git_err("find checkpoint commit"))?;
        let tree = commit.tree().map_err(git_err("read checkpoint tree"))?;
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| MemoryError::Checkpoint {
                op: "sweep the working tree",
                detail: "the city repository is bare".to_owned(),
            })?
            .to_path_buf();
        let mut deleted: Vec<String> = Vec::new();
        let mut fault: Option<MemoryError> = None;
        let walked = tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() != Some(git2::ObjectType::Blob) {
                return git2::TreeWalkResult::Ok;
            }
            let path = format!("{dir}{}", entry.name().unwrap_or_default());
            match std::fs::symlink_metadata(workdir.join(&path)) {
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => deleted.push(path),
                Err(err) => {
                    fault = Some(MemoryError::Checkpoint {
                        op: "sweep the working tree",
                        detail: format!("{path}: {err}"),
                    });
                    return git2::TreeWalkResult::Abort;
                }
            }
            git2::TreeWalkResult::Ok
        });
        walked.map_err(git_err("walk the checkpoint tree"))?;
        if let Some(err) = fault {
            return Err(err);
        }
        // Sorted here rather than trusted from the walk: the order these
        // records land in is part of what a replay reproduces.
        deleted.sort();
        let mut payloads = Vec::new();
        for path in deleted {
            let mut map = Map::new();
            map.insert(
                "paths".to_owned(),
                Value::Array(vec![Value::String(format!("file:{path}"))]),
            );
            let mut restoration = Map::new();
            restoration.insert(
                "tracked".to_owned(),
                Value::String(format!("file:{path}@{pre_oid}")),
            );
            map.insert("restoration".to_owned(), Value::Object(restoration));
            payloads.push(Payload::new(map).map_err(|source| MemoryError::Draft { source })?);
        }
        Ok(payloads)
    }

    /// The way back a `file_discarded` names: the blob at `address` in
    /// commit `oid`, written to the same path under the working tree.
    ///
    /// Neither the index nor HEAD moves: putting a file back is not a
    /// commit, and a person who restored one file should not find a new
    /// line in their own branch's history. `Address` cannot climb out of
    /// the city, so the path it spells needs no second check.
    ///
    /// # Errors
    /// A commit the repository no longer holds (the fence reference is
    /// what keeps one), a path that commit does not hold as a file, a
    /// bare repository, and a write the file system refuses.
    pub fn restore(&self, address: &Address, oid: &GitOid) -> Result<(), MemoryError> {
        let refused = |detail: String| MemoryError::Checkpoint {
            op: "restore a discarded file",
            detail,
        };
        let commit = git2::Oid::from_str(&oid.to_string())
            .and_then(|oid| self.repo.find_commit(oid))
            .map_err(git_err("find the restoration commit"))?;
        let blob = commit
            .tree()
            .and_then(|tree| tree.get_path(Path::new(address.as_str())))
            .and_then(|entry| entry.to_object(&self.repo))
            .map_err(git_err("find the discarded file in its commit"))?
            .into_blob()
            .map_err(|_| refused(format!("{address}@{oid} is not a file")))?;
        let target = self
            .repo
            .workdir()
            .ok_or_else(|| refused("the city repository is bare".to_owned()))?
            .join(address.as_str());
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| refused(format!("{}: {err}", parent.display())))?;
        }
        std::fs::write(&target, blob.content()).map_err(|err| refused(format!("{address}: {err}")))
    }
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
