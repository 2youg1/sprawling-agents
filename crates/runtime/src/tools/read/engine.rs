// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole text of a path a model chose, read through one buffered
//! open and judged where the handle it is read from really lies
//! (`crates/runtime/spec/Tools/Read.lean` §8-29-4-1, D10).
//!
//! The judged path (`chosen_path::land`, then `miss::text_at`) resolves
//! a path, opens it and resolves it again; on Windows each resolution is
//! itself an open, and a read paid four of them before the first byte.
//! Here the place the bytes come from is the place that is judged, so
//! there is no window between the two for a link to be swapped into.
//!
//! Every call this path cannot finish — an open, a metadata query, a
//! final-path query or a read that fails, a directory, bytes that are
//! not UTF-8 — is answered again by the judged path, whose refusals and
//! `nearby` lists are the ones a model has always been given. Only calls
//! that end in an error or a directory take it.

use kernel::{Address, AxError, ReadVerdict};

use super::miss::{self, Floor};
use crate::tools::chosen_path::{self, CityRoot};

/// The whole text at the admitted address `addr`, which the model wrote
/// as `asked`.
///
/// # Errors
/// `E_GATE_DENIED` when the file the handle reached lies outside the
/// city or in a place [`chosen_path::admit`] refuses; every other
/// refusal is the judged path's, byte for byte.
pub(super) fn text_of(
    city: &CityRoot,
    addr: &Address,
    asked: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<String, AxError> {
    match handle_read(city, addr, bound)? {
        Some(text) => Ok(text),
        None => judged_read(city, addr, asked, bound),
    }
}

/// The judged path: resolve, judge, open, judge the handle again.
pub(super) fn judged_read(
    city: &CityRoot,
    addr: &Address,
    asked: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<String, AxError> {
    let at = chosen_path::land(city, addr, "read", bound)?;
    miss::text_at(asked, at, &Floor::of_address(city.written(), addr))
}

/// Opens the file once and judges the handle where it lies (Windows).
/// `Ok(None)` hands the call to the judged path.
///
/// The open follows every link on the way, as the open inside
/// `std::fs::canonicalize` does on this platform, so it opens nothing
/// the judged path did not already open.
#[cfg(windows)]
fn handle_read(
    city: &CityRoot,
    addr: &Address,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Option<String>, AxError> {
    let Ok(file) = std::fs::File::open(city.join(addr)) else {
        return Ok(None);
    };
    let Ok(real) = winx::file::get_file_path(&file) else {
        return Ok(None);
    };
    city.judged_location(addr, &real, "read", bound)?;
    Ok(whole_text(file))
}

/// Resolves, judges, opens the judged location, and checks the handle
/// still lies there (Linux). `Ok(None)` hands the call to the judged
/// path.
///
/// Resolving first keeps the open after the judgement: a FIFO behind an
/// unjudged link would block an open made before it.
#[cfg(target_os = "linux")]
fn handle_read(
    city: &CityRoot,
    addr: &Address,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Option<String>, AxError> {
    use std::os::fd::AsRawFd as _;

    let Ok(real) = std::fs::canonicalize(city.join(addr)) else {
        return Ok(None);
    };
    city.judged_location(addr, &real, "read", bound)?;
    let Ok(file) = std::fs::File::open(&real) else {
        return Ok(None);
    };
    let fd = std::path::Path::new("/proc/self/fd").join(file.as_raw_fd().to_string());
    match std::fs::read_link(fd) {
        Ok(now) if now == real => Ok(whole_text(file)),
        // Moved since the judgement, or a machine without procfs: the
        // judged path checks the handle its own way and says which.
        Ok(_) | Err(_) => Ok(None),
    }
}

/// Every other platform reads through the judged path.
#[cfg(not(any(windows, target_os = "linux")))]
fn handle_read(
    _city: &CityRoot,
    _addr: &Address,
    _bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Option<String>, AxError> {
    Ok(None)
}

/// The whole file as text in one read, sized by the length the handle
/// reports; `None` for a directory, a failed read, or bytes that are
/// not UTF-8, which the judged path answers.
#[cfg(any(windows, target_os = "linux"))]
fn whole_text(mut file: std::fs::File) -> Option<String> {
    use std::io::Read as _;

    let meta = file.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    // One past the reported length, so a file that grew since the query
    // still ends in a single read rather than a reallocation.
    let capacity = usize::try_from(meta.len()).ok()?.checked_add(1)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes).ok()?;
    String::from_utf8(bytes).ok()
}
