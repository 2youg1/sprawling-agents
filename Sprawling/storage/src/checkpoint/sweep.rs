// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The post-wave sweep: what a checkpoint's tree holds and the working
//! tree no longer does, as `file_discarded` payloads.
//!
//! Specified by `crates/storage/spec/Checkpoint.lean` §8-8.

use kernel::Payload;
use serde_json::{Map, Value};

use crate::error::StorageError;

use super::commit::{Checkpoint, git_err};

impl Checkpoint {
    /// The post-wave sweep: every path present at `pre_oid` and gone
    /// from the working tree becomes a `file_discarded` payload whose
    /// restoration points back into that commit.
    ///
    /// **The question is existence, and only existence.** The tree is
    /// walked once and each blob is looked for on disk; a path that is
    /// not there is a deletion. Asking git for the tree-to-workdir diff
    /// instead would hash every path the tree and the worktree agree on,
    /// and the city itself keeps writing some of those paths after the
    /// checkpoint (a room's session projection, a worktree under a lease); a
    /// hash taken through a Windows directory entry that trails the open
    /// handle refuses the whole sweep. A sweep needs `Deleted` deltas,
    /// and a deletion is answered without reading any content.
    ///
    /// **The tree is the checkpoint's own write domain**, not the whole city:
    /// `wave_pre` walked exactly these paths before the wave began, so
    /// the walk costs what staging already cost.
    ///
    /// # Errors
    /// Propagates a `pre_oid` that is not an object, a commit whose tree
    /// cannot be read, and a working-tree path that cannot be examined.
    pub fn wave_post(&mut self, pre_oid: &str) -> Result<Vec<Payload>, StorageError> {
        let oid = git2::Oid::from_str(pre_oid).map_err(git_err("parse checkpoint oid"))?;
        let commit = self
            .repo
            .find_commit(oid)
            .map_err(git_err("find checkpoint commit"))?;
        let tree = commit.tree().map_err(git_err("read checkpoint tree"))?;
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| StorageError::Checkpoint {
                op: "sweep the working tree",
                detail: "the city repository is bare".to_owned(),
            })?
            .to_path_buf();
        let mut deleted: Vec<String> = Vec::new();
        let mut fault: Option<StorageError> = None;
        let walked = tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() != Some(git2::ObjectType::Blob) {
                return git2::TreeWalkResult::Ok;
            }
            // An address is UTF-8, so a name that is not cannot be swept
            // to a `file:` address; reading it as its directory would hide
            // a deletion.
            let Ok(name) = entry.name() else {
                fault = Some(StorageError::Checkpoint {
                    op: "sweep the working tree",
                    detail: format!(
                        "{dir}{}: the checkpoint holds a name that is not UTF-8",
                        String::from_utf8_lossy(entry.name_bytes())
                    ),
                });
                return git2::TreeWalkResult::Abort;
            };
            let path = format!("{dir}{name}");
            match std::fs::symlink_metadata(workdir.join(&path)) {
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => deleted.push(path),
                Err(err) => {
                    fault = Some(StorageError::Checkpoint {
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
            payloads.push(Payload::new(map).map_err(|source| StorageError::Draft { source })?);
        }
        Ok(payloads)
    }
}
