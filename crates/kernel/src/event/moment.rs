// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which lines record the moment their own event happened, read from the
//! line's kind and version (kernel-SPEC 8-4, "what the envelope `t`
//! records"). A reader asks here rather than comparing neighbouring
//! lines' `t` or guessing from the build that exported them.

use super::{EventKind, EventRecord, TimeMs};

impl EventKind {
    /// Whether a line of this kind records the moment its own event
    /// happened: a model attempt sent, a reply whole, a tool call started,
    /// a call's answer. The roster of those four kinds.
    pub fn records_a_moment(&self) -> bool {
        false
    }
}

impl EventRecord {
    /// This line's own moment: `Some(t)` for a kind that
    /// [`records_a_moment`](EventKind::records_a_moment), written at a
    /// version that measures it; `None` for every other line. A `None` on
    /// one of those four kinds means the moment was not measured, not
    /// that it took no time.
    pub fn moment(&self) -> Option<TimeMs> {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
