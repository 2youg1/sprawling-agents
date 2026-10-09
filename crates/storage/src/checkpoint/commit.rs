// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoints: base, wave pre/post.

use std::path::Path;

use kernel::event::record::{CheckpointCommitted, Commit, FileRestored};
use kernel::{Address, GitOid, Payload, TimeMs};

use crate::alias::WriteTarget;
use crate::bundle::landing::{Bits, land};
use crate::error::StorageError;
use crate::real_fs::RealFs;

use super::base::{BaseOf, BaseTarget};
use super::opening::HeadMove;
use super::provenance::Provenance;
use super::scan::CommitPlan;

/// How a checkpoint names itself in a commit subject. The whole set, because
/// a checkpoint that showed one of several prefixes would read like a checkpoint
/// that staged one of them.
pub(super) fn subject_of(scopes: &[String]) -> String {
    if scopes.is_empty() {
        return "checkpoint: .".to_owned();
    }
    format!("checkpoint: {}", scopes.join(" "))
}

/// Where a wave checkpoint is filed: under `refs/sprawling/`, which no
/// branch listing, push or `git log` walks by accident, and named by the
/// commit itself. A run checkpoints through more than one handle - the lane's
/// own and the bench's forecast net - and the reference is the only thing
/// that keeps a checkpoint from `git gc`, so its name cannot come from a count
/// that a second handle also keeps.
pub(super) fn checkpoint_ref(of: &Provenance, oid: git2::Oid) -> String {
    format!("refs/sprawling/runs/{}/{oid}", of.run())
}

/// The `checkpoint_committed` payload, in one place so a checkpoint and a
/// base commit cannot describe themselves differently.
///
/// It carries what the commit's own trailers carry that the record
/// cannot say for itself: the model and the effort. The run and the
/// actor are the record's identity and are not repeated here.
pub(super) fn committed(
    oid: git2::Oid,
    of: &Provenance,
    scopes: &[String],
    files: Vec<String>,
) -> Result<Payload, StorageError> {
    let spelled = oid.to_string();
    let oid = GitOid::parse(&spelled).ok_or_else(|| StorageError::Checkpoint {
        op: "record a checkpoint commit",
        detail: format!("git named this commit {spelled}, which is not 40 hex digits"),
    })?;
    let record = CheckpointCommitted::Committed(Commit {
        oid,
        by: of.attribution(),
        scope: scopes.to_vec(),
        files,
    });
    Payload::of(&record).map_err(|source| StorageError::Draft { source })
}

pub struct Checkpoint {
    pub(crate) repo: git2::Repository,
    /// The last commit this handle made, checkpoint or landing. The scan
    /// compares against it, because a checkpoint does not move HEAD.
    pub(crate) last: Option<git2::Oid>,
    /// Which index this handle stages into (`checkpoint::opening`).
    pub(crate) index: super::opening::IndexOwner,
}

pub(crate) fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> StorageError {
    move |err| StorageError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

impl Checkpoint {
    /// Makes sure the city has one commit, and makes no more than that.
    ///
    /// A worktree branches from a commit, so a city that has never been
    /// checkpointed cannot lend a tree. Committing on every dispatch would
    /// move the trunk under every request already waiting. Returns the
    /// commit it made, or `None` when there already was one.
    ///
    /// The commit is written the way [`Checkpoint::base_checkpoint`] writes
    /// one: every object in memory, then one pack, then the index and the
    /// branch, so a city of N files pays one file write for its objects
    /// rather than N, and a refusal leaves the index and HEAD as they were.
    ///
    /// # Errors
    /// Propagates whatever staging, scanning, packing and committing report.
    pub fn ensure_base(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Option<Payload>, StorageError> {
        if Self::head_commit(&self.repo)?.is_some() {
            return Ok(None);
        }
        let _head_moves = super::opening::moving_head()?;
        if Self::head_commit(&self.repo)?.is_some() {
            return Ok(None);
        }
        // The one checkpoint that moves the branch, written as one pack on a
        // second handle so this one keeps a disk-backed store (storage D27).
        let made =
            self.reopened()?
                .write_held(&BaseOf { scopes, t, of }, BaseTarget::Head, &mut |_| {});
        // The base wrote the index file through the other handle.
        self.repo
            .index()
            .and_then(|mut index| index.read(true))
            .map_err(git_err("read the index the base wrote"))?;
        match made {
            Ok(payload) => Ok(Some(payload)),
            // Another writer made the base between the read and the swap:
            // the same city as if it had gone first (storage §8-39).
            Err(_) if Self::head_commit(&self.repo)?.is_some() => Ok(None),
            Err(refused) => Err(refused),
        }
    }

    /// The pre-wave checkpoint: stage everything under `scopes`, scan it, and
    /// commit at the injected time. Returns the `checkpoint_committed`
    /// payload.
    ///
    /// **`scopes` is the run's write domain, not its room** (`crates/storage/spec/Checkpoint/Provenance.lean`
    /// §8-18). The two were allowed to differ once, and every
    /// file a resident wrote between them - a building's own documents,
    /// a second declared prefix - was staged by no checkpoint, reported by no
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
    ) -> Result<Payload, StorageError> {
        let files = self.stage_scopes(scopes)?;
        self.scan_staged()?;
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            head: HeadMove::Leave,
        })?;
        self.repo
            .reference(
                &checkpoint_ref(of, oid),
                oid,
                true,
                "sprawling: a wave checkpoint",
            )
            .map_err(git_err("file a wave checkpoint"))?;
        committed(oid, of, scopes, files)
    }

    /// The way back a `file_discarded` names: the blob at `address` in
    /// commit `oid`, written to the same path under the working tree.
    ///
    /// Neither the index nor HEAD moves: putting a file back is not a
    /// commit, and a person who restored one file should not find a new
    /// line in their own branch's history. A link on the path, or a file
    /// already at the target with other bytes, is refused rather than
    /// followed or overwritten.
    ///
    /// # Errors
    /// A commit the repository no longer holds (the checkpoint reference is
    /// what keeps one), a path that commit does not hold as a file, a
    /// bare repository, an address in the protected metadata subtree
    /// (`Address::is_reserved`), and a write the file system refuses;
    /// `StorageError::Alias` when a symbolic link or junction sits on the
    /// path below the working tree, because `Address` bounds the
    /// spelling of a path and not where the disk resolves it.
    pub fn restore(&self, address: &Address, oid: &GitOid) -> Result<(), StorageError> {
        let refused = |detail: String| StorageError::Checkpoint {
            op: "restore a discarded file",
            detail,
        };
        if address.is_reserved() {
            return Err(refused(format!(
                "{address} is protected metadata; the city writes it through its own gates, not a restore"
            )));
        }
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
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| refused("the city repository is bare".to_owned()))?;
        let target = WriteTarget::within(
            "restore a discarded file",
            workdir,
            &workdir.join(address.as_str()),
        )?;
        let target = target.as_path();
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| refused(format!("{}: {err}", parent.display())))?;
        }
        match std::fs::read(target) {
            Ok(current) if current == blob.content() => return Ok(()),
            Ok(_) => {
                return Err(refused(format!(
                    "{address}: a file already sits there; move it aside and restore again"
                )));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(refused(format!("{address}: {err}"))),
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .and_then(|mut file| std::io::Write::write_all(&mut file, blob.content()))
            .map_err(|err| refused(format!("{address}: {err}")))
    }

    /// Takes `address` back from `point` into the city's own working
    /// tree: the point's bytes in place of what is there, or no file when
    /// the point holds none (`crates/storage/spec/Checkpoint.lean` §8-33). Returns the
    /// `file_restored` record of this step, `name` empty for this tree.
    ///
    /// # Errors
    /// What [`Checkpoint::restore`] refuses for the path and the commit,
    /// something other than a file at `address` in `point`, and a write
    /// or a removal the file system refuses.
    pub fn take_back(
        &self,
        address: &Address,
        point: &GitOid,
    ) -> Result<FileRestored, StorageError> {
        let refused = |detail: String| StorageError::Checkpoint {
            op: "take a file back from a checkpoint",
            detail,
        };
        if address.is_reserved() {
            return Err(refused(format!(
                "{address} is protected metadata; the city writes it through its own gates"
            )));
        }
        let tree = git2::Oid::from_str(&point.to_string())
            .and_then(|oid| self.repo.find_commit(oid))
            .and_then(|commit| commit.tree())
            .map_err(git_err("find the checkpoint to take a file back from"))?;
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| refused("the city repository is bare".to_owned()))?;
        let target = WriteTarget::within(
            "take a file back from a checkpoint",
            workdir,
            &workdir.join(address.as_str()),
        )?;
        let restored = FileRestored {
            name: String::new(),
            path: address.as_str().to_owned(),
            point: *point,
        };
        let entry = match tree.get_path(Path::new(address.as_str())) {
            Ok(entry) => entry,
            // The point never held it: taking it back is taking it away.
            Err(err) if err.code() == git2::ErrorCode::NotFound => {
                return match std::fs::remove_file(target.as_path()) {
                    Ok(()) => Ok(restored),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(restored),
                    Err(err) => Err(refused(format!("{address}: {err}"))),
                };
            }
            Err(err) => return Err(git_err("find the file in the checkpoint")(err)),
        };
        let blob = entry
            .to_object(&self.repo)
            .map_err(git_err("read the file in the checkpoint"))?
            .into_blob()
            .map_err(|_| refused(format!("{address}@{point} is not a file")))?;
        if let Some(parent) = target.as_path().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| refused(format!("{}: {err}", parent.display())))?;
        }
        land(&mut RealFs::new(), target, blob.content(), Bits::OfReplaced)?;
        Ok(restored)
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
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod restore_tests;
