// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use kernel::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use serde_json::Value;
fn record(run: RunId, seq: u64, kind: EventKind, data: Value) -> EventRecord {
    let map = data.as_object().cloned().unwrap_or_default();
    let draft = EventDraft {
        run,
        t: TimeMs::new(seq.saturating_mul(10)),
        who: "resident".to_owned(),
        addr: None,
        kind,
        data: Payload::new(map).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
}
fn script(run: RunId) -> Vec<EventRecord> {
    vec![
        record(run, 0, EventKind::RunStarted, serde_json::json!({})),
        record(
            run,
            1,
            EventKind::FileDiscarded,
            serde_json::json!({
                "paths": ["file:build/out.o", "file:build/tmp.o"],
                "restoration": { "rebuildable": { "reason": "cargo build" } },
            }),
        ),
        record(run, 2, EventKind::ToolCalled, serde_json::json!({})),
        record(
            run,
            3,
            EventKind::DiscardRestored,
            serde_json::json!({ "discard_seq": 1 }),
        ),
        record(
            run,
            4,
            EventKind::RunFrozen,
            serde_json::json!({ "completion": "done" }),
        ),
    ]
}

#[test]
fn two_rebuilds_from_one_ledger_export_the_same_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let run = RunId::from_bytes([5u8; 16]);
    let records = script(run);

    let (mut first, _) = Projection::open(&tmp.path().join("a.redb")).unwrap();
    for r in &records {
        first.apply(r).unwrap();
    }
    let first_bytes = first.export_canonical().unwrap();

    // A second projection, built from the same ledger in the same
    // order, is indistinguishable at the logical level.
    let (mut second, _) = Projection::open(&tmp.path().join("b.redb")).unwrap();
    for r in &records {
        second.apply(r).unwrap();
    }
    assert_eq!(first_bytes, second.export_canonical().unwrap());

    // And a discarded file that came back reads as restored.
    let bin = first.recycle_bin().unwrap();
    assert_eq!(bin.len(), 1);
    assert_eq!(bin[0].seq, Seq::new(1));
    assert_eq!(bin[0].paths.len(), 2);
    assert_eq!(bin[0].restoration, "rebuildable");
    assert!(bin[0].restored);

    let rows = first.run_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].started_t, TimeMs::new(0));
    assert_eq!(rows[0].frozen.as_deref(), Some("done"));
}

#[test]
fn replaying_the_ledger_twice_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let run = RunId::from_bytes([6u8; 16]);
    let records = script(run);
    let (mut projection, _) = Projection::open(&tmp.path().join("p.redb")).unwrap();
    for r in &records {
        projection.apply(r).unwrap();
    }
    let once = projection.export_canonical().unwrap();
    for r in &records {
        projection.apply(r).unwrap();
    }
    assert_eq!(once, projection.export_canonical().unwrap());
    assert_eq!(projection.last_applied(), Some(Seq::new(4)));
}

#[test]
fn a_reopened_projection_resumes_where_it_stopped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("resume.redb");
    let run = RunId::from_bytes([7u8; 16]);
    let records = script(run);
    {
        let (mut projection, _) = Projection::open(&path).unwrap();
        for r in records.iter().take(3) {
            projection.apply(r).unwrap();
        }
        assert_eq!(projection.last_applied(), Some(Seq::new(2)));
    }
    let (mut reopened, _) = Projection::open(&path).unwrap();
    assert_eq!(
        reopened.last_applied(),
        Some(Seq::new(2)),
        "the resume point survives the restart"
    );
    for r in &records {
        reopened.apply(r).unwrap();
    }
    assert_eq!(reopened.last_applied(), Some(Seq::new(4)));

    // Deleting the file and replaying everything gives the same view.
    let fresh_path = tmp.path().join("fresh.redb");
    let (mut fresh, _) = Projection::open(&fresh_path).unwrap();
    for r in &records {
        fresh.apply(r).unwrap();
    }
    assert_eq!(
        reopened.export_canonical().unwrap(),
        fresh.export_canonical().unwrap(),
        "the projection is derived, so it is disposable"
    );
}

/// The SPEC states this view's recovery as "delete the file and
/// replay", and the whole argument for keeping a young store here is
/// that its failure costs a rebuild rather than data. That argument
/// is worth nothing while no code performs the rebuild: the first
/// caller to meet a file it cannot read would have met an error
/// instead.
#[test]
fn an_unreadable_view_is_rebuilt_rather_than_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("garbled.redb");
    std::fs::write(&path, b"this is not a redb file, and never was").unwrap();

    let (mut projection, report) =
        Projection::open(&path).expect("a derived view opens even when its file does not");
    let rebuilt = report
        .rebuilt
        .expect("open must say that it reset the view, not reset it in silence");
    assert!(
        !rebuilt.reason.is_empty(),
        "removing the file destroys the only other copy of this sentence"
    );
    assert_eq!(
        projection.last_applied(),
        None,
        "an empty view is the instruction to replay from the start"
    );

    let run = RunId::from_bytes([9u8; 16]);
    let records = script(run);
    for r in &records {
        projection.apply(r).unwrap();
    }
    let fresh_path = tmp.path().join("fresh-after-garble.redb");
    let (mut fresh, fresh_report) = Projection::open(&fresh_path).unwrap();
    assert!(
        fresh_report.rebuilt.is_none(),
        "a path with no file is the ordinary first run, not a repair"
    );
    for r in &records {
        fresh.apply(r).unwrap();
    }
    assert_eq!(
        projection.export_canonical().unwrap(),
        fresh.export_canonical().unwrap(),
        "the rebuilt view is the view"
    );
}

#[test]
fn a_restore_naming_no_discard_is_dropped_not_invented() {
    let tmp = tempfile::tempdir().unwrap();
    let run = RunId::from_bytes([8u8; 16]);
    let (mut projection, _) = Projection::open(&tmp.path().join("q.redb")).unwrap();
    projection
        .apply(&record(
            run,
            0,
            EventKind::RunStarted,
            serde_json::json!({}),
        ))
        .unwrap();
    projection
        .apply(&record(
            run,
            1,
            EventKind::DiscardRestored,
            serde_json::json!({ "discard_seq": 4242 }),
        ))
        .unwrap();
    assert!(projection.recycle_bin().unwrap().is_empty());
    assert_eq!(projection.last_applied(), Some(Seq::new(1)));
}
