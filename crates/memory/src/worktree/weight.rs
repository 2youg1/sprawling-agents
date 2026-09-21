// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a working tree weighs, for the ceiling a claim is judged
//! against.

use std::path::Path;

use kernel::ByteLen;

use crate::error::MemoryError;
use crate::reserved::outside_reserved;

/// Bytes of the work a person did under a directory: git's own
/// bookkeeping and the city's reserved subtree both excluded.
///
/// The ceiling asks how big the working tree a node would copy is, and
/// the ledger, the object store, the projections and the other nodes'
/// trees are none of it. Counting them meant a city that had run for a
/// month refused to hand out a tree because its own history had grown,
/// and said so in a sentence about the working tree.
///
/// An explicit worklist rather than recursion: a deep tree is a
/// data-dependent depth, and a stack overflow is not a failure a caller
/// can handle.
///
/// # Errors
/// Propagates a directory that exists and cannot be read. A directory
/// that is not there weighs nothing, which is what a tree not yet
/// created weighs.
pub(super) fn measure(root: &Path) -> Result<ByteLen, MemoryError> {
    let mut total: u64 = 0;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(MemoryError::Io {
                    op: "measure a worktree",
                    path: dir.clone(),
                    source,
                });
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue; // a link's target is measured where it lives
            }
            if kind.is_dir() {
                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }
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
    Ok(ByteLen::new(total))
}
