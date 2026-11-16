// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn init_writes_genesis_and_refuses_a_second_birth() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    assert_eq!(report.genesis.seq(), kernel::Seq::FIRST);
    assert_eq!(report.genesis.kind(), EventKind::CityInitialized);

    runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();

    let err = init_city(dir.path()).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
}

#[test]
fn the_startup_scan_closes_dangling_calls_once_and_reports_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let raised = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();

    let run = RunId::from_bytes([7u8; 16]);
    let mut data = serde_json::Map::new();
    data.insert(
        "id".to_owned(),
        serde_json::Value::String("tu_9".to_owned()),
    );
    data.insert(
        "name".to_owned(),
        serde_json::Value::String("edit".to_owned()),
    );
    data.insert(
        "args".to_owned(),
        serde_json::Value::Object(serde_json::Map::new()),
    );
    worker
        .record_for(
            run,
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::ToolCalled,
                data: Payload::new(data).unwrap(),
            },
        )
        .unwrap();
    let report = worker.startup_scan().unwrap();
    assert_eq!(report.closed_calls, 1, "the dangling call is closed");

    let again = worker.startup_scan().unwrap();
    assert_eq!(again.closed_calls, 0, "the repair is idempotent");
    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let closed = verified.lines().iter().any(|line| match line {
        runtime::replay::VerifiedLine::Known { record, .. } => {
            record.kind() == EventKind::ToolResult
                && serde_json::to_string(record.data())
                    .unwrap()
                    .contains("E_TOOL_OUTCOME_UNKNOWN")
        }
        _ => false,
    });
    assert!(closed, "the closing result states the unknown outcome");
}

#[test]
fn an_old_shaped_dangling_call_is_closed_and_never_run() {
    let dir = tempfile::tempdir().unwrap();
    let raised = init_city(dir.path()).unwrap();
    let notes = dir.path().join("lab/room1/notes.md");
    std::fs::create_dir_all(notes.parent().unwrap()).unwrap();
    std::fs::write(&notes, "draft\n").unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    let older = serde_json::json!({
        "id": "tu_old",
        "name": "edit",
        "args": { "path": "lab/room1/notes.md", "old": "draft", "new": "final" },
        "subject": "lab/room1/notes.md",
    });
    worker
        .record_for(
            RunId::from_bytes([8u8; 16]),
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::ToolCalled,
                data: Payload::of(&older).unwrap(),
            },
        )
        .unwrap();
    let report = worker.startup_scan().unwrap();
    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let closing = verified.lines().iter().rev().find_map(|line| match line {
        runtime::replay::VerifiedLine::Known { record, .. }
            if record.kind() == EventKind::ToolResult =>
        {
            Some(serde_json::to_value(record.data()).unwrap())
        }
        _ => None,
    });
    assert_eq!(
        (
            report.closed_calls,
            closing.map(|data| (data["tool_use_id"].clone(), data["error"]["code"].clone())),
            std::fs::read_to_string(&notes).unwrap(),
        ),
        (
            1,
            Some((
                serde_json::json!("tu_old"),
                serde_json::json!("E_TOOL_OUTCOME_UNKNOWN")
            )),
            "draft\n".to_owned(),
        )
    );
}

#[test]
fn a_new_city_delegates_its_approvals_to_the_clerk_on_the_record() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        hands(),
    )
    .unwrap();
    assert_eq!(
        worker.governance.autonomy,
        kernel::Autonomy::Delegate(
            kernel::ResidentId::new(kernel::consts_policy::HALL_CLERK).unwrap()
        ),
        "the clerk answers from the moment the city exists, folded back from the ledger"
    );
}

#[test]
fn a_ledger_that_cannot_be_listed_is_an_error_not_an_empty_city() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = kernel::layout::CityLayout::new(dir.path()).ledger();
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::fs::write(&ledger, b"not a directory").unwrap();
    assert!(has_history(dir.path()).is_err());
}
