// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a working tree weighs, for the ceiling a claim is judged
//! against, and what a checkout that has just finished wrote.

use std::path::Path;

use kernel::ByteLen;

use super::trees::open_tree;
use crate::error::StorageError;
use crate::reserved::outside_reserved;

/// What a walk under a directory found: the bytes, and how many
/// directory entries it read to find them.
pub(super) struct Weight {
    pub(super) bytes: ByteLen,
    pub(super) walked: u64,
}

/// What a checkout wrote into a new tree, read back from the index it
/// wrote.
pub(super) struct Written {
    pub(super) files: u64,
    pub(super) bytes: ByteLen,
}

/// The mode git gives a submodule's entry: a commit, not a file the
/// checkout writes.
const GITLINK: u32 = 0o160_000;

/// Reads back what the checkout of a new tree wrote, from the index it
/// wrote: libgit2 lstats every file it writes and records the size in
/// that file's entry, so the sum is a measure of the disk rather than
/// an estimate, and no walk of the tree is needed to find it
/// (`crates/storage/spec/Worktree.lean` §8-31). A submodule's entry is a commit, not a file.
///
/// # Errors
/// Propagates a tree whose repository or index cannot be opened.
pub(super) fn written(tree: &git2::Worktree) -> Result<Written, StorageError> {
    let refuse = |op: &'static str, err: git2::Error| StorageError::Worktree {
        op,
        detail: format!("{}: {err}", tree.path().display()),
    };
    let repo = open_tree(tree).map_err(|err| refuse("open a new worktree", err))?;
    let index = repo
        .index()
        .map_err(|err| refuse("read a new worktree's index", err))?;
    let mut files: u64 = 0;
    let mut bytes: u64 = 0;
    for entry in index.iter().filter(|entry| entry.mode != GITLINK) {
        files = files.saturating_add(1);
        bytes = bytes.saturating_add(u64::from(entry.file_size));
    }
    Ok(Written {
        files,
        bytes: ByteLen::new(bytes),
    })
}

/// Bytes of the work a person did under a directory: git's own
/// bookkeeping and the city's reserved subtree both excluded.
///
/// The ceiling asks how big the working tree a node would copy is, and
/// the ledger, the object store, the projections and the other nodes'
/// trees are none of it. Counting them would make a city's own history
/// refuse it a tree, in a sentence about the working tree.
///
/// An explicit worklist rather than recursion: a deep tree is a
/// data-dependent depth, and a stack overflow is not a failure a caller
/// can handle.
///
/// # Errors
/// Propagates a directory that exists and cannot be read. A directory
/// that is not there weighs nothing, which is what a tree not yet
/// created weighs.
pub(super) fn measure(root: &Path) -> Result<Weight, StorageError> {
    let mut total: u64 = 0;
    let mut walked: u64 = 0;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(StorageError::Io {
                    op: "measure a worktree",
                    path: dir.clone(),
                    source,
                });
            }
        };
        for entry in entries.flatten() {
            walked = walked.saturating_add(1);
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue; // a link's target is measured where it lives
            }
            if kind.is_dir() {
                // `outside_reserved` names git's own metadata beside the
                // city's reserved subtree, so what is bookkeeping rather
                // than work has one predicate here as everywhere.
                let ours = path
                    .strip_prefix(root)
                    .is_ok_and(|relative| !outside_reserved(relative));
                if ours {
                    continue;
                }
                pending.push(path);
            } else if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Ok(Weight {
        bytes: ByteLen::new(total),
        walked,
    })
}
