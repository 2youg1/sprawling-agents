// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole text of a path a model chose, read through one buffered
//! open and judged where the handle it is read from really lies
//! (`crates/runtime/spec/Tools/Read.lean` §8-29-4-1, D113).
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

use kernel::{Address, AxError, B3Hash, ReadVerdict};

use super::miss::{self, Floor};
use crate::tools::chosen_path::{self, CityRoot};
use crate::tools::shown::{Answer, ReadShown, Recall, Span};
use crate::tools::stamp::Stamp;

/// One chosen-path read: its judgement inputs and the window evidence
/// that may answer it. These values travel through every fast-path arm.
#[derive(Clone, Copy)]
pub(super) struct Request<'a> {
    pub(super) city: &'a CityRoot,
    pub(super) addr: &'a Address,
    pub(super) asked: &'a str,
    pub(super) bound: &'a dyn Fn(&Address) -> ReadVerdict,
    pub(super) shown: &'a ReadShown,
    pub(super) want: Span,
}

/// A file's text, or a reference to an interval already in the window.
/// Both arms carry the judged address and the handle's stamp.
pub(super) enum Looked {
    Text {
        text: String,
        before: Stamp,
        after: Stamp,
        addr: Address,
    },
    Unchanged {
        stamp: Stamp,
        addr: Address,
        answer: Answer,
    },
}

impl Looked {
    fn text(text: String, before: Stamp, after: Stamp, addr: Address) -> Looked {
        Looked::Text {
            text,
            before,
            after,
            addr,
        }
    }
}

/// The whole text at the admitted address `addr`, which the model wrote
/// as `asked`, or an unchanged reply backed by confirmed evidence.
///
/// # Errors
/// `E_GATE_DENIED` when the file the handle reached lies outside the
/// city or in a place [`chosen_path::admit`] refuses; every other
/// refusal is the judged path's, byte for byte.
pub(super) fn text_of(request: Request<'_>) -> Result<Looked, AxError> {
    match handle_read(request)? {
        Some(looked) => Ok(looked),
        None => judged_read(request.city, request.addr, request.asked, request.bound),
    }
}

/// The judged path: resolve, judge, open, judge the handle again. It
/// always returns text; without the one-handle fast path it cannot make
/// an unchanged claim.
pub(super) fn judged_read(
    city: &CityRoot,
    addr: &Address,
    asked: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Looked, AxError> {
    let judged = chosen_path::land(city, addr, "read", bound)?;
    let text = miss::text_at(
        asked,
        judged.location,
        &Floor::of_address(city.written(), addr),
    )?;
    Ok(Looked::text(text.0, text.1, text.2, judged.addr))
}

/// Opens the file once and judges the handle where it lies (Windows).
/// `Ok(None)` hands the call to the judged path.
///
/// The open follows every link on the way, as the open inside
/// `std::fs::canonicalize` does on this platform, so it opens nothing
/// the judged path did not already open.
#[cfg(windows)]
fn handle_read(request: Request<'_>) -> Result<Option<Looked>, AxError> {
    let Ok(file) = std::fs::File::open(request.city.join(request.addr)) else {
        return Ok(None);
    };
    let Ok(real) = winx::file::get_file_path(&file) else {
        return Ok(None);
    };
    let landed = request
        .city
        .judged_location(request.addr, &real, "read", request.bound)?;
    Ok(read_handle(file, landed, request))
}

/// Resolves, judges, opens the judged location, and checks the handle
/// still lies there (Linux). `Ok(None)` hands the call to the judged
/// path.
///
/// Resolving first keeps the open after the judgement: a FIFO behind an
/// unjudged link would block an open made before it.
#[cfg(target_os = "linux")]
fn handle_read(request: Request<'_>) -> Result<Option<Looked>, AxError> {
    use std::os::fd::AsRawFd as _;

    let Ok(real) = std::fs::canonicalize(request.city.join(request.addr)) else {
        return Ok(None);
    };
    let landed = request
        .city
        .judged_location(request.addr, &real, "read", request.bound)?;
    let Ok(file) = std::fs::File::open(&real) else {
        return Ok(None);
    };
    let fd = std::path::Path::new("/proc/self/fd").join(file.as_raw_fd().to_string());
    match std::fs::read_link(fd) {
        Ok(now) if now == real => Ok(read_handle(file, landed, request)),
        // Moved since the judgement, or a machine without procfs: the
        // judged path checks the handle its own way and says which.
        Ok(_) | Err(_) => Ok(None),
    }
}

/// Every other platform reads through the judged path.
#[cfg(not(any(windows, target_os = "linux")))]
fn handle_read(_request: Request<'_>) -> Result<Option<Looked>, AxError> {
    Ok(None)
}

/// Decides from the handle's first facts whether any bytes are needed.
/// `Repeat` reads none. `Compare` reads once and returns a stub only
/// when the stamps stayed equal and the complete content hash matches.
#[cfg(any(windows, target_os = "linux"))]
fn read_handle(file: std::fs::File, addr: Address, request: Request<'_>) -> Option<Looked> {
    let facts = Stamp::facts(&file).ok()?;
    if !facts.is_file() {
        return None;
    }
    let before = facts.stamp().clone();
    match request
        .shown
        .recall(&addr, &before, request.asked, request.want)
    {
        Recall::Fresh => read_text(file, facts.len(), before)
            .map(|(text, before, after)| Looked::text(text, before, after, addr)),
        Recall::Repeat(answer) => Some(Looked::Unchanged {
            stamp: before,
            addr,
            answer,
        }),
        Recall::Compare { digest, answer } => {
            read_text(file, facts.len(), before).map(|(text, before, after)| {
                if before == after && B3Hash::digest(text.as_bytes()) == digest {
                    Looked::Unchanged {
                        stamp: after,
                        addr,
                        answer,
                    }
                } else {
                    Looked::text(text, before, after, addr)
                }
            })
        }
    }
}

/// Reads the file once, sized by the length the handle reports, with a
/// stamp on each side; `None` for a failed read or query or non-UTF-8
/// bytes, which the judged path answers.
#[cfg(any(windows, target_os = "linux"))]
fn read_text(mut file: std::fs::File, len: u64, before: Stamp) -> Option<(String, Stamp, Stamp)> {
    use std::io::Read as _;

    // One past the reported length, so a file that grew since the query
    // still ends in a single read rather than a reallocation.
    let capacity = usize::try_from(len).ok()?.checked_add(1)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes).ok()?;
    let after = Stamp::of_file(&file).ok()?;
    Some((String::from_utf8(bytes).ok()?, before, after))
}
