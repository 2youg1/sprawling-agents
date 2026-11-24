// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::{CheckpointCommitted, CheckpointFinding, Commit, FileRestored};
use kernel::{Address, GitOid, Payload, TimeMs};

use crate::alias::WriteTarget;
use crate::bundle::landing::{Bits, land};
use crate::error::StorageError;
use crate::real_fs::RealFs;

use super::base::{BaseOf, BaseTarget};
use super::opening::HeadMove;
use super::provenance::Provenance;
use super::scan::CommitPlan;

pub(super) fn subject_of(scopes: &[String]) -> String {
    if scopes.is_empty() {
        return "checkpoint: .".to_owned();
    }
    format!("checkpoint: {}", scopes.join(" "))
}

pub(super) fn checkpoint_ref(of: &Provenance, oid: git2::Oid) -> String {
    format!("refs/sprawling/runs/{}/{oid}", of.run())
}

pub(super) struct CommitDetails<'a> {
    pub(super) of: &'a Provenance,
    pub(super) scopes: &'a [String],
    pub(super) files: Vec<String>,
    pub(super) withheld: Vec<CheckpointFinding>,
}

pub(super) fn committed(
    oid: git2::Oid,
    details: CommitDetails<'_>,
) -> Result<Payload, StorageError> {
    let record = commit_record(oid, details)?;
    Payload::of(&CheckpointCommitted::Committed(record))
        .map_err(|source| StorageError::Draft { source })
}

pub(super) fn commit_record(
    oid: git2::Oid,
    details: CommitDetails<'_>,
) -> Result<Commit, StorageError> {
    let spelled = oid.to_string();
    let oid = GitOid::parse(&spelled).ok_or_else(|| StorageError::Checkpoint {
        op: "record a checkpoint commit",
        detail: format!("git named this commit {spelled}, which is not 40 hex digits"),
    })?;
    Ok(Commit {
        oid,
        by: details.of.attribution(),
        scope: details.scopes.to_vec(),
        files: details.files,
        withheld: details.withheld,
    })
}

pub struct Checkpoint {
    pub(crate) repo: git2::Repository,

    pub(crate) last: Option<git2::Oid>,

    pub(crate) index: super::opening::IndexOwner,
    pub(crate) reference: Option<String>,

    pub(crate) entropy_allow: kernel::EntropyAllowlist,
}

pub(crate) fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> StorageError {
    move |err| StorageError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

impl Checkpoint {
    pub fn ensure_base(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Option<Commit>, StorageError> {
        if Self::head_commit(&self.repo)?.is_some() {
            return Ok(None);
        }
        let _head_moves = super::opening::moving_head(self.repo.path())?;
        if Self::head_commit(&self.repo)?.is_some() {
            return Ok(None);
        }

        let made = self.reopened()?.write_held(
            &BaseOf {
                scopes,
                t,
                of,
                source: None,
            },
            BaseTarget::Head,
            &mut |_| {},
        );

        self.repo
            .index()
            .and_then(|mut index| index.read(true))
            .map_err(git_err("read the index the base wrote"))?;
        match made {
            Ok(record) => Ok(Some(record)),

            Err(_) if Self::head_commit(&self.repo)?.is_some() => Ok(None),
            Err(refused) => Err(refused),
        }
    }

    pub fn wave_pre(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Payload, StorageError> {
        let files = self.stage_scopes(scopes)?;
        let withheld = self.scan_staged(scopes)?;
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
        committed(
            oid,
            CommitDetails {
                of,
                scopes,
                files,
                withheld,
            },
        )
    }

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
