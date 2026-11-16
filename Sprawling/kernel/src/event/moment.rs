// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{EventKind, EventRecord, TimeMs};
use crate::consts_external::EVENT_LOG_V;

const MOMENTS_SINCE_V: u32 = 2;

const _: () = assert!(MOMENTS_SINCE_V <= EVENT_LOG_V);

impl EventKind {

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

    pub fn moment(&self) -> Option<TimeMs> {
        (self.v() >= MOMENTS_SINCE_V && self.kind().records_a_moment()).then(|| self.t())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
