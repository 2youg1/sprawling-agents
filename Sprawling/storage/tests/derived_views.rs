// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "test code: this suite names the one event kind it is               about and treats the rest as one arm, rather than               modelling an enum it does not assert on"
)]

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::fmt::Debug;
use std::path::Path;

use kernel::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use proptest::prelude::*;
use storage::{HotView, LedgerIndex};

fn rebuild_is_stable<V, O: PartialEq + Debug>(
    records: &[EventRecord],
    build: impl Fn(&[EventRecord], u8) -> V,
    observe: impl Fn(&V) -> O,
) {
    let first = build(records, 0);
    let second = build(records, 1);
    assert_eq!(
        observe(&first),
        observe(&second),
        "the same ledger must rebuild to the same view"
    );
}

fn record(run: RunId, seq: u64, kind: EventKind, data: serde_json::Value) -> EventRecord {
    let map = data.as_object().cloned().unwrap_or_default();
    let draft = EventDraft {
        run,
        t: TimeMs::new(seq.saturating_mul(7)),
        who: format!("resident-{}", seq % 3),
        addr: None,
        kind,
        data: Payload::new(map).expect("payload"),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b"prev"))
}

fn kind_of(pick: u8) -> EventKind {
    match pick % 4 {
        0 => EventKind::RunStarted,
        1 => EventKind::FileDiscarded,
        2 => EventKind::RunFrozen,
        _ => EventKind::ToolCalled,
    }
}

fn script(picks: &[(u8, u8)]) -> Vec<EventRecord> {
    picks
        .iter()
        .enumerate()
        .map(|(i, (kind_pick, run_pick))| {
            let run = RunId::from_bytes([run_pick.saturating_add(1); 16]);
            let seq = u64::try_from(i).unwrap_or(0);
            let kind = kind_of(*kind_pick);
            let data = match kind {
                EventKind::FileDiscarded => serde_json::json!({
                    "paths": [format!("file:work/{seq}.tmp")],
                    "restoration": { "interred": format!("cas:b3-{seq}") },
                }),
                EventKind::RunFrozen => serde_json::json!({ "completion": "done" }),
                _ => serde_json::json!({}),
            };
            record(run, seq, kind, data)
        })
        .collect()
}

fn write_segment(dir: &Path, records: &[EventRecord]) {
    let mut blob = Vec::new();
    for r in records {
        blob.extend_from_slice(&r.canonical_line().expect("canonical line"));
        blob.push(b'\n');
    }
    std::fs::write(dir.join("ledger-00000000000000000000.jsonl"), &blob).expect("write segment");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn every_derived_view_rebuilds_to_the_same_answer(
        picks in prop::collection::vec((0u8..4, 0u8..3), 1..24)
    ) {
        let records = script(&picks);
        let root = tempfile::tempdir().expect("tempdir");

        rebuild_is_stable(
            &records,
            |records, nonce| {
                let dir = root.path().join(format!("idx-{nonce}"));
                std::fs::create_dir_all(&dir).expect("mkdir");
                write_segment(&dir, records);
                let index = LedgerIndex::rebuild(&dir).expect("index");
                (dir, index)
            },
            |(dir, index)| {
                let mut reader = index.reader(dir);
                let lines: Vec<Vec<u8>> = (0..index.len())
                    .map(|i| {
                        reader
                            .line_at(Seq::new(u64::try_from(i).unwrap_or(0)))
                            .expect("line")
                    })
                    .collect();
                (index.tail_seq(), lines)
            },
        );

        rebuild_is_stable(
            &records,
            |records, _| {
                let mut view = HotView::new();
                for r in records {
                    view.apply(r).expect("apply");
                }
                view
            },
            |view| {
                view.runs()
                    .map(|(id, hot)| format!("{id} {:?} {} {:?}", hot.phase, hot.last_seq.value(), hot.last_kind))
                    .collect::<Vec<String>>()
            },
        );
    }
}
