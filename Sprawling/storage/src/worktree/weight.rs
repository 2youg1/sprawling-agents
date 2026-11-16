// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::ByteLen;

use super::trees::open_tree;
use crate::error::StorageError;
use crate::reserved::outside_reserved;

pub(super) struct Weight {
    pub(super) bytes: ByteLen,
    pub(super) walked: u64,
}

pub(super) struct Written {
    pub(super) files: u64,
    pub(super) bytes: ByteLen,
}

const GITLINK: u32 = 0o160_000;

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
                continue;
            }
            if kind.is_dir() {
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
