// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which lines record the moment their own event happened, read from the
//! line's kind and version (`crates/kernel/spec/Event.lean` §8-4, "what the envelope `t`
//! records"). A reader asks here rather than comparing neighbouring
//! lines' `t` or guessing from the build that exported them.

use super::{EventKind, EventRecord, TimeMs};
use crate::consts_external::EVENT_LOG_V;

/// The first ledger version whose waited-for lines record their own
/// moment; a line written before carries its turn's stamp there.
const MOMENTS_SINCE_V: u32 = 2;

// A build that writes an older version than the one moments start at
// would answer `None` for every line it writes itself.
const _: () = assert!(MOMENTS_SINCE_V <= EVENT_LOG_V);

impl EventKind {
    /// Whether a line of this kind records the moment its own event
    /// happened: a model attempt sent, a reply whole, a tool call started,
    /// a call's answer. The roster of those four kinds.
    pub fn records_a_moment(&self) -> bool {
        matches!(
            self,
            EventKind::ModelCalled
                | EventKind::ModelReturned
                | EventKind::ToolCalled
                | EventKind::ToolResult
        )
    }
}

impl EventRecord {
    /// This line's own moment: `Some(t)` for a kind that
    /// [`records_a_moment`](EventKind::records_a_moment), written at a
    /// version that measures it; `None` for every other line. A `None` on
    /// one of those four kinds means the moment was not measured, not
    /// that it took no time.
    pub fn moment(&self) -> Option<TimeMs> {
        (self.v() >= MOMENTS_SINCE_V && self.kind().records_a_moment()).then(|| self.t())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
