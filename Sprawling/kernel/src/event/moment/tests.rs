// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A waited-for line reads as its moment from version two on, and as
//! unmeasured on a line written before.

use super::*;
use crate::consts_external::EVENT_LOG_V;
use crate::event::{EventDraft, Payload, RunId, Seq};
use crate::ledger::GENESIS_PREV;

fn line(kind: EventKind, t: u64) -> EventRecord {
    let draft = EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: "resident@sim.1".into(),
        addr: None,
        kind,
        data: Payload::empty(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::FIRST, GENESIS_PREV)
}

#[test]
fn a_waited_for_line_reads_as_its_moment_from_version_two_on() {
    let answered = line(EventKind::ToolResult, 7);
    assert_eq!(answered.moment(), Some(TimeMs::new(7)));

    let written = String::from_utf8(answered.canonical_line().unwrap()).unwrap();
    let current = format!("\"v\":{EVENT_LOG_V},");
    assert!(written.starts_with(&format!("{{{current}")), "{written}");
    let older = written.replacen(&current, "\"v\":1,", 1);
    let read = EventRecord::parse_line(older.as_bytes()).unwrap();
    assert_eq!(read.moment(), None, "a v1 line's moment was not measured");

    assert_eq!(line(EventKind::PromptAssembled, 7).moment(), None);

    let recording: Vec<EventKind> = EventKind::ALL
        .into_iter()
        .filter(EventKind::records_a_moment)
        .collect();
    assert_eq!(
        recording,
        vec![
            EventKind::ModelCalled,
            EventKind::ModelReturned,
            EventKind::ToolCalled,
            EventKind::ToolResult,
        ]
    );
}
