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

/// A claim's line is written when the model makes it, so a run whose
/// landing fails before its plan settles still owes the history the
/// line that closes it: the claim, then the node handed back
/// (sprawling-SPEC.md 8-42-8). The goal's line is the one lost here
/// because landing writes it before the plan's.
#[test]
fn a_claim_whose_landing_failed_is_handed_back() {
    use crate::assembly::fixture::*;
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_ONE_FREE_ROW,
    )
    .unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "taking a row",
                "tu_1",
                "plan",
                serde_json::json!({ "action": "claim", "node": "1" }),
            ),
            tool_completion(
                "staking ground",
                "tu_2",
                "goal",
                serde_json::json!({
                    "statement": "rewrite the kiln notes",
                    "paths": ["lab/room1/notes.md"],
                    "standing": true,
                }),
            ),
            completion("done", None),
        ],
    );
    let fs = memory::FaultFs::new(memory::FaultPlan {
        cut_at_op: None,
        cut_on_write: Some("goal_registered"),
        torn_tail: memory::TornTail::None,
    });
    let opened = memory::JsonlLedger::open_faulty(
        fs,
        &kernel::layout::CityLayout::new(dir.path()).ledger(),
        now_ms().unwrap(),
    )
    .unwrap();
    let worker = RunWorker::over(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
        opened,
    )
    .unwrap();
    let mut worker = attach_provider(worker, &base_url, "m-local").unwrap();
    let landed = worker.handle(channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "take a row and stake the notes".to_owned(),
        goal: "one claim, one goal".to_owned(),
        mode: kernel::Mode::PlanGoal,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lost goal"),
        session: None,
        effort: None,
    });
    drop(provider);

    let closing: Vec<String> = worker
        .ledger
        .read_raw_lines()
        .unwrap()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter_map(|line| line["kind"].as_str().map(str::to_owned))
        .filter(|kind| kind.starts_with("roadmap_"))
        .collect();
    assert_eq!(
        (landed.is_err(), closing),
        (
            true,
            vec!["roadmap_claimed".to_owned(), "roadmap_released".to_owned()]
        ),
        "the landing failed ahead of the plan, and the claim it booked is handed back"
    );
}
