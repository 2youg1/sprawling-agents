// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A `search` answer as one header line and its entries grouped by file.
//!
//! The header says what the answer holds before any of it is read, so a
//! model that asked too broad a question learns that from the first
//! line. Hit lines carry the marks `read` draws (`super::read`), because
//! a passage copied out of a search answer goes to `edit` as readily as
//! one copied out of a read, and `edit` sees through only that grammar.

use kernel::Payload;
use serde_json::{Map, Value};

use super::read::push_mark;

/// The header and the grouped entries of one `search` payload, or `None`
/// when the payload is not the shape `tools::search` writes.
pub(super) fn text(result: &Payload) -> Option<String> {
    let map = result.as_map();
    let mut body = String::new();
    let mut header = if let Some(matches) = map.get("matches") {
        content(matches.as_array()?, &mut body)?
    } else if let Some(files) = map.get("files") {
        files_only(files.as_array()?, &mut body)?
    } else {
        counts(map.get("counts")?.as_array()?, &mut body)?
    };
    if let Some(stopped) = map.get("stopped") {
        header.push_str("; ");
        header.push_str(stopped.as_str()?);
    }
    let unreadable = map.get("unreadable").map_or(Some(0), Value::as_u64)?;
    if unreadable > 0 {
        header.push_str(&format!("; {unreadable} not searched"));
        unread(map, &mut body)?;
    }
    header.push('\n');
    header.push_str(&body);
    Some(header)
}

/// Every hit, a group per file: the path on a line of its own, then its
/// hit blocks, the hit line marked and so is the block's first line when
/// context starts above it. Groups are parted by an empty line.
fn content(matches: &[Value], body: &mut String) -> Option<String> {
    let mut files = 0_usize;
    let mut last: Option<&str> = None;
    for hit in matches {
        let path = hit.get("path")?.as_str()?;
        let line = hit.get("line")?.as_u64()?;
        let from = hit.get("from").map_or(Some(line), Value::as_u64)?;
        if last != Some(path) {
            if last.is_some() {
                body.push('\n');
            }
            body.push_str(path);
            body.push('\n');
            files = files.saturating_add(1);
            last = Some(path);
        }
        for (at, text) in (from..).zip(hit.get("text")?.as_str()?.split('\n')) {
            if at == from || at == line {
                push_mark(body, at);
            }
            body.push_str(text);
            body.push('\n');
        }
    }
    Some(format!(
        "{} in {}",
        counted(matches.len(), "match", "matches"),
        counted(files, "file", "files")
    ))
}

/// One path per line.
fn files_only(files: &[Value], body: &mut String) -> Option<String> {
    for path in files {
        body.push_str(path.as_str()?);
        body.push('\n');
    }
    Some(counted(files.len(), "file", "files"))
}

/// One `path:count` per line, and the total in the header.
fn counts(counts: &[Value], body: &mut String) -> Option<String> {
    let mut total = 0_u64;
    for entry in counts {
        let count = entry.get("count")?.as_u64()?;
        body.push_str(&format!("{}:{count}", entry.get("path")?.as_str()?));
        body.push('\n');
        total = total.saturating_add(count);
    }
    Some(format!(
        "{}, {}",
        counted(counts.len(), "file", "files"),
        counted(usize::try_from(total).ok()?, "match", "matches")
    ))
}

/// The places the walk could not look through, one per line with its
/// reason, after an empty line.
fn unread(map: &Map<String, Value>, body: &mut String) -> Option<()> {
    let named = map
        .get("unread")
        .map_or(Some(&[][..]), |list| list.as_array().map(Vec::as_slice))?;
    if !named.is_empty() {
        body.push('\n');
    }
    for place in named {
        body.push_str(&format!(
            "{} not searched: {}\n",
            place.get("path")?.as_str()?,
            place.get("why")?.as_str()?
        ));
    }
    Some(())
}

/// `n` with the noun that agrees with it.
fn counted(n: usize, one: &str, many: &str) -> String {
    match n {
        1 => format!("1 {one}"),
        _ => format!("{n} {many}"),
    }
}
