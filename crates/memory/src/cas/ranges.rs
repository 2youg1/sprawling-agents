// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Locator range grammar, read off a stored object without lifting
//! the object.
//!
//! One authority for what `L` and `B` mean: `B` is a 0-based closed byte
//! range, `L` a 1-based closed line range whose answer keeps the
//! newlines between the lines it names and drops the terminator after
//! the last of them. Out of bounds refuses; nothing is ever clamped.
//!
//! **A range read does not verify the object.** Its address covers the
//! whole object, so the only way to check a fragment against it is to
//! read the whole object and hash it, which is the cost this module
//! exists to remove. Integrity is bought at `put` and re-checked by
//! [`crate::Cas::get`]; a caller that needs the address proved takes the
//! full read (memory-SPEC 8-3).

use std::path::Path;

use kernel::{B3Hash, Range};

use crate::error::{MemoryError, io_err};
use crate::vfs::Vfs;

/// How much of an object one read lifts while walking to a line.
///
/// Large enough that a line near the front of a large object costs one
/// read, small enough that the walk never holds a megabyte it will
/// discard.
pub(crate) const SCAN_CHUNK_BYTES: u64 = 64 * 1024;

/// The bytes `range` names inside the object at `path`.
pub(crate) fn of_object(
    vfs: &dyn Vfs,
    path: &Path,
    hash: &B3Hash,
    range: &Range,
) -> Result<Vec<u8>, MemoryError> {
    match range {
        Range::Bytes { from, to } => bytes_of(vfs, path, hash, (*from, *to)),
        Range::Lines { from, to } => lines_of(vfs, path, hash, (*from, *to)),
    }
}

fn out_of_bounds(hash: &B3Hash) -> MemoryError {
    MemoryError::RangeOutOfBounds {
        hash: hash.to_string(),
    }
}

/// A 0-based closed byte range, read where it sits.
///
/// A short answer is how the end of the object announces itself: the
/// seam returns what exists, and a request that reaches past it is out
/// of bounds rather than a shorter answer.
fn bytes_of(
    vfs: &dyn Vfs,
    path: &Path,
    hash: &B3Hash,
    span: (u64, u64),
) -> Result<Vec<u8>, MemoryError> {
    let (from, to) = span;
    let wanted = to
        .checked_sub(from)
        .and_then(|width| width.checked_add(1))
        .ok_or_else(|| out_of_bounds(hash))?;
    let bytes = vfs
        .read_at(path, from, wanted)
        .map_err(io_err("read cas range", path))?;
    let got = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if got != wanted {
        return Err(out_of_bounds(hash));
    }
    Ok(bytes)
}

/// A 1-based closed line range, found by scanning for newlines from the
/// front of the object and stopping at the terminator of the last line
/// asked for.
///
/// The scan costs the bytes before the answer, never the bytes after
/// it: a line near the front of a large object is one read.
fn lines_of(
    vfs: &dyn Vfs,
    path: &Path,
    hash: &B3Hash,
    span: (u64, u64),
) -> Result<Vec<u8>, MemoryError> {
    let (from, to) = span;
    if from == 0 || from > to {
        return Err(out_of_bounds(hash));
    }
    let mut offset: u64 = 0;
    // The line the next byte belongs to, and whether that line has
    // received any byte yet - together they say how many lines the
    // object holds once the scan reaches its end.
    let mut line: u64 = 1;
    let mut line_started = false;
    let mut picked: Vec<u8> = Vec::new();
    loop {
        let chunk = vfs
            .read_at(path, offset, SCAN_CHUNK_BYTES)
            .map_err(io_err("read cas range", path))?;
        let read = u64::try_from(chunk.len()).unwrap_or(u64::MAX);
        for byte in &chunk {
            if *byte != b'\n' {
                line_started = true;
                if line >= from && line <= to {
                    picked.push(*byte);
                }
                continue;
            }
            if line == to {
                return Ok(picked);
            }
            if line >= from {
                picked.push(b'\n');
            }
            line = line.saturating_add(1);
            line_started = false;
        }
        if read < SCAN_CHUNK_BYTES {
            break;
        }
        offset = offset.saturating_add(read);
    }
    // The object ended. An unterminated last line still counts as one,
    // which is why the scan tracked whether the current line had begun.
    let count = if line_started {
        line
    } else {
        line.saturating_sub(1)
    };
    if to > count {
        return Err(out_of_bounds(hash));
    }
    Ok(picked)
}
