// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The first line of a segment, read without reading the segment.
//!
//! The version probe and the city's identity both need line 1 alone,
//! and a segment grows to tens of megabytes: lifting it whole to split
//! off one line made a single-segment ledger cost two full reads on
//! every open (`crates/storage/spec/Jsonl.lean` §8-1). The window starts at one page and
//! doubles, so a line of any length costs at most twice its own bytes.

use std::io;
use std::path::Path;

use crate::vfs::Vfs;

const FIRST_WINDOW_BYTES: u64 = 4096;

/// The first `\n`-terminated line of `path`, without its `\n`.
///
/// `None` when the file holds no terminated line: an empty segment, or
/// one whose first line is torn, which is tail recovery's to judge.
///
/// # Errors
/// Propagates a read the filesystem refuses.
pub(crate) fn first_line(vfs: &dyn Vfs, path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut window = FIRST_WINDOW_BYTES;
    loop {
        let offset = u64::try_from(line.len()).unwrap_or(u64::MAX);
        let chunk = vfs.read_at(path, offset, window)?;
        if let Some(end) = chunk.iter().position(|byte| *byte == b'\n') {
            line.extend(chunk.iter().take(end));
            return Ok(Some(line));
        }
        if u64::try_from(chunk.len()).unwrap_or(u64::MAX) < window {
            return Ok(None);
        }
        line.extend_from_slice(&chunk);
        window = window.saturating_mul(2);
    }
}
