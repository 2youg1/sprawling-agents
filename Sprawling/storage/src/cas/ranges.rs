// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{B3Hash, Range};

use crate::error::{StorageError, io_err};
use crate::vfs::Vfs;

pub(crate) const SCAN_CHUNK_BYTES: u64 = 64 * 1024;

pub(crate) fn of_object(
    vfs: &dyn Vfs,
    path: &Path,
    hash: &B3Hash,
    range: &Range,
) -> Result<Vec<u8>, StorageError> {
    match range {
        Range::Bytes { from, to } => bytes_of(vfs, path, hash, (*from, *to)),
        Range::Lines { from, to } => lines_of(vfs, path, hash, (*from, *to)),
    }
}

fn out_of_bounds(hash: &B3Hash) -> StorageError {
    StorageError::RangeOutOfBounds {
        hash: hash.to_string(),
    }
}

fn bytes_of(
    vfs: &dyn Vfs,
    path: &Path,
    hash: &B3Hash,
    span: (u64, u64),
) -> Result<Vec<u8>, StorageError> {
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

fn lines_of(
    vfs: &dyn Vfs,
    path: &Path,
    hash: &B3Hash,
    span: (u64, u64),
) -> Result<Vec<u8>, StorageError> {
    let (from, to) = span;
    if from == 0 || from > to {
        return Err(out_of_bounds(hash));
    }
    let mut offset: u64 = 0;

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
