// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The dormant index: one line per namespace of the admitted tools a
//! session does not carry, and one per admitted skill, packed greedily
//! under
//! [`DORMANT_INDEX_CEILING`](super::DORMANT_INDEX_CEILING)
//! (`crates/runtime/spec/Catalog.lean` `pack`, `index_within_ceiling`).
//!
//! Each entry is tried with its hint, then with its name alone, and the
//! first entry that fits neither way ends the list with `+N more`. The
//! budget reserves that last line at the largest count before any entry
//! is placed, so it always fits; the header says once that hints are cut,
//! because a marker on every cut hint would cost more than the hint.

use super::DORMANT_INDEX_CEILING;
use crate::elision::Boundary;

/// How long one entry's hint may be, in bytes, before it is cut on a
/// character boundary.
pub(crate) const HINT_MAX_BYTES: usize = 64;

const HEADER: &str = "Dormant, admitted here and not loaded (hints cut), one line per \
                      namespace: `tool` lists a namespace, finds by a few words, details a \
                      tool and runs it; open a skill with `read`.\n";

/// One capability as the index lists it: the label it is called by and
/// the text its hint is cut from.
pub(super) struct Entry {
    pub(super) label: String,
    pub(super) about: String,
}

/// The index for `entries`, in their order; empty when there are none.
pub(super) fn index(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut room = DORMANT_INDEX_CEILING
        .saturating_sub(HEADER.len())
        .saturating_sub(more(entries.len()).len());
    let mut out = String::from(HEADER);
    for (placed, entry) in entries.iter().enumerate() {
        let hinted = format!("- {}: {}\n", entry.label, hint(&entry.about));
        let bare = format!("- {}\n", entry.label);
        let Some(line) = [hinted, bare].into_iter().find(|line| line.len() <= room) else {
            out.push_str(&more(entries.len().saturating_sub(placed)));
            return out;
        };
        room = room.saturating_sub(line.len());
        out.push_str(&line);
    }
    out
}

/// The line that counts the entries left out.
fn more(left: usize) -> String {
    format!("+{left} more\n")
}

/// The first sentence of `about`, on its first line, cut to
/// [`HINT_MAX_BYTES`] on a character boundary. `tool find` reads the
/// whole text, so what is cut here is still found.
pub(super) fn hint(about: &str) -> &str {
    let line = about.split_once('\n').map_or(about, |(first, _)| first);
    let sentence = line
        .split_once(". ")
        .map_or(line, |(first, _)| first)
        .trim_end_matches('.')
        .trim();
    Boundary::before(sentence, HINT_MAX_BYTES).head()
}
