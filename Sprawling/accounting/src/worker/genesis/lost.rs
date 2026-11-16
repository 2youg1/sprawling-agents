// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::event::record::RunFrozen;
use kernel::{AxError, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};

#[derive(Debug, Default)]
pub(super) struct OpenRuns {
    open: BTreeMap<RunId, Open>,
}

#[derive(Debug)]
struct Open {
    started: Seq,
    who: String,
}

impl OpenRuns {

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

    pub(super) fn into_drafts(self, t: TimeMs) -> Result<Vec<EventDraft>, AxError> {
        let mut open: Vec<(RunId, Open)> = self.open.into_iter().collect();
        open.sort_by_key(|(_, one)| one.started);
        open.into_iter()
            .map(|(run, one)| {
                Ok(EventDraft {
                    run,
                    t,
                    who: one.who,

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
