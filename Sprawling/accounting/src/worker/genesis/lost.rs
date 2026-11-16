// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The runs a process death left open, and the line that freezes each
//! one (`crates/accounting/spec/Worker/Genesis/Lost.lean` §8-18-1, ARCHITECTURE.md 13.7 `Lost --> Frozen`).
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests {
    use kernel::{Address, EventKind, Payload, RunId};

    use crate::worker::RunWorker;
    use crate::worker::fixture::{hands, init_city};

    /// A run the process died in has its opening line and no freeze: the
    /// scan closes its call, then freezes it cancelled with the cause, as
    /// the resident whose lines it wrote; a second scan writes nothing
    /// (`crates/accounting/spec/Worker/Genesis/Lost.lean` §8-18-1).
    #[test]
    fn a_run_the_process_died_in_is_frozen_once() {
        let dir = tempfile::tempdir().unwrap();
        let raised = init_city(dir.path()).unwrap();
        let mut worker = RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            hands(),
        )
        .unwrap();
        let run = RunId::from_bytes([7u8; 16]);
        let room = Address::parse("lab/room1").unwrap();
        let started = kernel::event::record::RunStarted {
            task: "measure the meter".to_owned(),
            ..kernel::event::record::RunStarted::default()
        };
        worker
            .record_for(
                run,
                crate::effect::Line {
                    who: "city".to_owned(),
                    addr: room.clone(),
                    kind: EventKind::RunStarted,
                    data: Payload::of(&started).unwrap(),
                },
            )
            .unwrap();
        worker
            .record_for(
                run,
                crate::effect::Line {
                    who: "lab/room1".to_owned(),
                    addr: room,
                    kind: EventKind::ToolCalled,
                    data: Payload::of(&serde_json::json!({
                        "id": "tu_9",
                        "name": "status",
                        "args": {},
                    }))
                    .unwrap(),
                },
            )
            .unwrap();

        worker.startup_scan().unwrap();
        worker.startup_scan().unwrap();

        let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
        let written: Vec<(EventKind, String, serde_json::Value)> = verified
            .lines()
            .iter()
            .filter_map(|line| match line {
                runtime::replay::VerifiedLine::Known { record, .. } if record.run() == run => {
                    Some((
                        record.kind(),
                        record.who().to_owned(),
                        serde_json::to_value(record.data()).unwrap(),
                    ))
                }
                _ => None,
            })
            .skip(2)
            .map(|(kind, who, data)| match kind {
                EventKind::RunFrozen => (kind, who, data),
                _ => (kind, who, serde_json::Value::Null),
            })
            .collect();
        assert_eq!(
            written,
            vec![
                (
                    EventKind::ToolResult,
                    "lab/room1".to_owned(),
                    serde_json::Value::Null
                ),
                (
                    EventKind::RunFrozen,
                    "lab/room1".to_owned(),
                    serde_json::json!({ "completion": "cancelled", "cause": "process_died" })
                ),
            ]
        );
    }
}
