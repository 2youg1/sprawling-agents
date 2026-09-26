// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::assembly::*;

/// A claim lands through the run that made it (`record_for`), and the
/// worker's own table of who holds which node reads it at once, as a
/// restart folding the same history does (sprawling-SPEC.md 8-91).
#[test]
fn a_claim_a_run_lands_reaches_the_holders_the_worker_reads() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let mut claim = serde_json::Map::new();
    claim.insert("by".to_owned(), "lab/room1".into());
    claim.insert("node".to_owned(), "2".into());
    claim.insert("verb".to_owned(), "claimed".into());
    worker
        .record_for(
            RunId::from_bytes([7; 16]),
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::RoadmapClaimed,
                data: Payload::new(claim).unwrap(),
            },
        )
        .unwrap();
    let building = Address::parse("lab").unwrap();
    let rebuilt = Standing::fold(&report.ledger_dir)
        .unwrap()
        .collaboration
        .plan_holders
        .in_building(&building);
    assert_eq!(
        rebuilt.values().cloned().collect::<Vec<String>>(),
        vec!["lab/room1".to_owned()]
    );
    assert_eq!(
        worker.holders_in(&building),
        rebuilt,
        "the live holders and the restart's holders are one fold"
    );
}
