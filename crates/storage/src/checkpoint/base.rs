// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A building's base checkpoint, written as one pack.

use std::io::Write;

use kernel::{Payload, TimeMs};

use crate::error::StorageError;

use super::commit::{Checkpoint, checkpoint_ref, committed, git_err, subject_of};
use super::provenance::Provenance;
use super::scan::CommitPlan;

/// How far a base checkpoint has got, reported as each step lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseProgress {
    /// The building's files are in the index and their blobs in memory.
    Staged { files: usize },
    /// The one pack holding every object the checkpoint wrote is on disk.
    Packed { bytes: usize },
}

impl Checkpoint {
    /// Stages `scopes`, scans them and commits, with every object held
    /// in memory until the end and then written as one pack.
    ///
    /// Takes the handle by value: the in-memory store is installed on
    /// this handle's object database and cannot be taken off again, so
    /// the handle goes with it, and no later checkpoint can write an object
    /// into a store nobody dumps (`crates/storage/spec/Checkpoint.lean` §8-8).
    ///
    /// The commit moves HEAD when the city has none, and is otherwise
    /// filed under `refs/sprawling/runs/<run>/<oid>` like a wave checkpoint.
    ///
    /// # Errors
    /// Propagates staging, the staged-secret scan, the commit, a pack the
    /// object database refuses, and a reference it will not write.
    pub fn base_checkpoint(
        mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        progress: &mut dyn FnMut(BaseProgress),
    ) -> Result<Payload, StorageError> {
        let objects = self.repo.path().join("objects");
        let objects = objects.to_str().ok_or_else(|| StorageError::Checkpoint {
            op: "open the object store",
            detail: format!("{} is not valid UTF-8", objects.display()),
        })?;
        // The disk store is an alternate, and libgit2 writes to no
        // alternate: every object this checkpoint makes lands in memory.
        let odb = git2::Odb::new().map_err(git_err("open an in-memory object store"))?;
        let held = odb
            .add_new_mempack_backend(1)
            .map_err(git_err("open an in-memory object store"))?;
        odb.add_disk_alternate(objects)
            .map_err(git_err("read the object store"))?;
        self.repo
            .set_odb(&odb)
            .map_err(git_err("hold the base checkpoint in memory"))?;

        let files = self.stage_scopes_held(scopes)?;
        progress(BaseProgress::Staged { files: files.len() });
        self.scan_staged()?;
        // The same question `ensure_base` asks: a city with no HEAD has
        // no base yet, and this commit becomes it.
        let onto_head = self.repo.head().is_err();
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            onto_head: false,
        })?;

        let mut pack = git2::Buf::new();
        held.dump(&self.repo, &mut pack)
            .map_err(git_err("dump the base checkpoint"))?;
        let disk =
            git2::Repository::open(self.repo.path()).map_err(git_err("open the object store"))?;
        let disk_odb = disk.odb().map_err(git_err("open the object store"))?;
        let mut writer = disk_odb
            .packwriter()
            .map_err(git_err("write the base checkpoint pack"))?;
        writer
            .write_all(&pack)
            .map_err(|err| StorageError::Checkpoint {
                op: "write the base checkpoint pack",
                detail: err.to_string(),
            })?;
        writer
            .commit()
            .map_err(git_err("write the base checkpoint pack"))?;
        progress(BaseProgress::Packed { bytes: pack.len() });

        // Only now are the objects on disk, so only now may the index and
        // a reference name them: a failure above leaves both as they were.
        super::scan::write_index(&mut self.repo.index().map_err(git_err("read index"))?)?;
        let name = if onto_head {
            self.repo
                .find_reference("HEAD")
                .map_err(git_err("read HEAD"))?
                .symbolic_target()
                .map_err(|err| StorageError::Checkpoint {
                    op: "read HEAD",
                    detail: err.to_string(),
                })?
                .map(str::to_owned)
                .ok_or_else(|| StorageError::Checkpoint {
                    op: "move HEAD to the base checkpoint",
                    detail: "HEAD names no branch".to_owned(),
                })?
        } else {
            checkpoint_ref(of, oid)
        };
        self.repo
            .reference(&name, oid, true, "sprawling: a base checkpoint")
            .map_err(git_err("file a base checkpoint"))?;
        held.reset()
            .map_err(git_err("release the base checkpoint"))?;
        committed(oid, of, scopes, files)
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
