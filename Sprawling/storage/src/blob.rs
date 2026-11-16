// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes of one file as one commit holds them (`crates/storage/spec/Blob.lean` §8-29). The working tree and the index are not consulted: a
//! `file:<addr>@<oid>` Locator names a commit's bytes, and the file on
//! disk may have moved on since.

use std::path::Path;

use kernel::{Address, GitOid};

use crate::StorageError;
use crate::checkpoint::commit::git_err;

/// The blob at `addr` in the tree of commit `oid`, or `None` when that
/// place in the commit is not a file (a directory, a submodule, nothing).
///
/// # Errors
/// `StorageError::Checkpoint` when the city repository does not open, the
/// commit is not in it, or the blob does not read.
pub fn blob_at(
    city_root: &Path,
    oid: GitOid,
    addr: &Address,
) -> Result<Option<Vec<u8>>, StorageError> {
    let repo = git2::Repository::open(city_root).map_err(git_err("open the city repository"))?;
    let commit = git2::Oid::from_str(&oid.to_string())
        .and_then(|parsed| repo.find_commit(parsed))
        .map_err(git_err("find the commit a Locator names"))?;
    let tree = commit
        .tree()
        .map_err(git_err("read the tree of a commit"))?;
    let entry = match tree.get_path(Path::new(addr.as_str())) {
        Ok(entry) => entry,
        Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(err) => return Err(git_err("find a path in a commit's tree")(err)),
    };
    match entry.kind() {
        Some(git2::ObjectType::Blob) => repo
            .find_blob(entry.id())
            .map(|blob| Some(blob.content().to_vec()))
            .map_err(git_err("read a blob at a commit")),
        Some(_) | None => Ok(None),
    }
}
