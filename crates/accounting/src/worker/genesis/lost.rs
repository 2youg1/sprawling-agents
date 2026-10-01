// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The runs a process death left open, and the line that freezes each
//! one (accounting-SPEC.md 8-18-1, ARCHITECTURE.md 13.7 `Lost --> Frozen`).
//!
//! The scan that reads them runs only once the worker holds the writer's
//! lock and before it drives anything, so a run with a `run_started` and
//! no `run_frozen` is one the last process was driving when it died: no
//! process will ever freeze it but this one.

use std::collections::BTreeMap;

use kernel::event::record::RunFrozen;
use kernel::{AxError, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};

/// Every run still open in the records observed so far: the seq of its
/// `run_started`, which orders the freezes, and the author of its latest
/// line, which writes them.
#[derive(Debug, Default)]
pub(super) struct OpenRuns {
    open: BTreeMap<RunId, Open>,
}

/// One open run, as far as the freeze needs it.
#[derive(Debug)]
struct Open {
    started: Seq,
    who: String,
}

impl OpenRuns {
    /// Reads one record, in ledger order.
    ///
    /// The author follows the run's latest line because `run_started` is
    /// written as the city and every later line as the run's resident;
    /// the freeze is counted against the resident, as the closing
    /// `tool_result` of a dangling call is.
    pub(super) fn observe(&mut self, record: &EventRecord) {
        let run = record.run();
        let kind = record.kind();
        if kind == EventKind::RunStarted {
            self.open.insert(
                run,
                Open {
                    started: record.seq(),
                    who: record.who().to_owned(),
                },
            );
        } else if kind == EventKind::RunFrozen {
            self.open.remove(&run);
        } else if let Some(open) = self.open.get_mut(&run) {
            record.who().clone_into(&mut open.who);
        }
    }

    /// One `run_frozen` per run still open, stamped `t`, in the order the
    /// runs started.
    ///
    /// # Errors
    /// Propagates a freeze the payload refuses, which `RunFrozen::lost`
    /// never builds.
    pub(super) fn into_drafts(self, t: TimeMs) -> Result<Vec<EventDraft>, AxError> {
        let mut open: Vec<(RunId, Open)> = self.open.into_iter().collect();
        open.sort_by_key(|(_, one)| one.started);
        open.into_iter()
            .map(|(run, one)| {
                Ok(EventDraft {
                    run,
                    t,
                    who: one.who,
                    // As `runtime::run::Charter::close` writes a freeze:
                    // the room is on the run's `run_started`.
                    addr: None,
                    kind: EventKind::RunFrozen,
                    data: Payload::of(&RunFrozen::lost())?,
                    ig: false,
                })
            })
            .collect()
    }
}
