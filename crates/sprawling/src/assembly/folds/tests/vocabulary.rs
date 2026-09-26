// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A history that verifies is a history a city starts from.

use super::super::*;
use crate::assembly::*;

/// A newer writer's line that marks itself ignorable passes the one
/// per-line check; the folds read what that check read, so the city
/// opens past it instead of refusing a line it was told it may skip.
#[test]
fn a_city_opens_past_an_ignorable_line_from_a_newer_vocabulary() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let segment = memory::ledger_segments_at(&report.ledger_dir)
        .unwrap()
        .pop()
        .unwrap();
    let mut bytes = std::fs::read(&segment).unwrap();
    let lines: Vec<&[u8]> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    let future = format!(
        "{{\"v\":1,\"run\":\"00000000-0000-0000-0000-000000000000\",\"seq\":{},\
         \"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\
         \"data\":{{}},\"ig\":true}}\n",
        lines.len(),
        kernel::ledger::chain_hash(lines.last().unwrap())
    );
    bytes.extend_from_slice(future.as_bytes());
    std::fs::write(&segment, &bytes).unwrap();

    let views = rebuild_views(&report.ledger_dir).map(|_| ());
    let standing = Standing::fold(&report.ledger_dir).map(|_| ());
    assert_eq!((views, standing), (Ok(()), Ok(())));
}

/// Serving reads the history once; what it hands the pages and the
/// worker is what a read for each would have folded, and what the
/// worker that wrote the history holds.
#[test]
fn one_read_of_the_history_folds_what_a_read_for_each_would() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::Halt {
            scope: channels::HaltScope::City,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"halt"),
        })
        .unwrap();

    let judged = |governance: &Governance| {
        (
            governance.halted.clone(),
            governance.autonomy.clone(),
            governance.granted.clone(),
        )
    };
    // The worker holds the writer lock; `fold_city` takes it before it reads.
    let the_worker_judged = judged(&worker.governance);
    drop(worker);

    let (mut views, (_ledger, _report, standing)) = fold_city(&report.ledger_dir).unwrap();
    // Answered from what `Views::apply` folded, not from the on-disk index.
    for applied in [
        channels::Query::CityView,
        channels::Query::Governance,
        channels::Query::ApprovalQueue,
    ] {
        assert_eq!(
            views.answer(&applied),
            rebuild_views(&report.ledger_dir).unwrap().answer(&applied)
        );
    }
    assert_eq!(
        judged(&standing.governance),
        judged(&Standing::fold(&report.ledger_dir).unwrap().governance)
    );
    assert_eq!(
        judged(&standing.governance),
        the_worker_judged,
        "and the comparison above is not two empty folds agreeing"
    );
    assert!(!standing.governance.halted.is_empty());
}

/// A pursuit line this build cannot read stops the fold. Skipping it
/// would leave standing whatever goal the line changed, so a building a
/// person cleared would go on pursuing after a restart.
#[test]
fn a_pursuit_line_the_build_cannot_read_stops_the_fold() {
    let line = unreadable(
        kernel::EventKind::PursuitChanged,
        serde_json::json!({ "step": "abandon", "goal": "read the meter" }),
    );
    assert!(CollaborationFold::default().absorb(&line).is_err());
}

fn unreadable(kind: kernel::EventKind, data: serde_json::Value) -> EventRecord {
    let draft = kernel::EventDraft {
        run: RunId::CITY,
        t: kernel::TimeMs::new(0),
        who: "person".into(),
        addr: Some(Address::parse("lab").unwrap()),
        kind,
        data: kernel::Payload::new(data.as_object().unwrap().clone()).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, kernel::Seq::FIRST, kernel::ledger::GENESIS_PREV)
}
