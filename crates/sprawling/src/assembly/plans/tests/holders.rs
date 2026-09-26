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
    let claim = kernel::event::record::RoadmapMoved {
        by: "lab/room1".to_owned(),
        node: kernel::NodeId::parse("2").unwrap(),
        step: kernel::event::record::RoadmapStep::Claimed {
            item: "the kiln notes".to_owned(),
        },
    };
    worker
        .record_for(
            RunId::from_bytes([7; 16]),
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::RoadmapClaimed,
                data: Payload::of(&claim).unwrap(),
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

/// A claim booked at the call reaches the ledger through the gate the
/// lanes write through, and the worker's own holders read it before the
/// claiming run lands, as a restart folding the same history does
/// (sprawling-SPEC.md 8-42-8, 8-90).
#[test]
fn a_claim_booked_through_the_gate_reaches_the_live_holders_before_its_run_lands() {
    use kernel::Tool;
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let (building, room) = (
        Address::parse("lab").unwrap(),
        Address::parse("lab/room1").unwrap(),
    );
    let claimant = booking::Claimant {
        building: building.clone(),
        room: room.clone(),
        run: RunId::from_bytes([7; 16]),
        who: "potter@lab.7".to_owned(),
        clock: std::sync::Arc::new(SystemClock),
    };
    let desk = collab::ClaimDesk::new(
        "potter@lab.7".to_owned(),
        room,
        crate::assembly::fixture::PLAN_ONE_FREE_ROW.to_owned(),
        booking::booking(worker.bell(), claimant),
    );
    let tool = collab::ClaimTool::new(std::sync::Arc::new(std::sync::Mutex::new(desk))).unwrap();
    let call = kernel::ToolCall {
        id: "tu_1".to_owned(),
        name: kernel::ToolName::parse("plan").unwrap(),
        args: Payload::new(
            serde_json::json!({ "action": "claim", "node": "1" })
                .as_object()
                .unwrap()
                .clone(),
        )
        .unwrap(),
    };
    let lane = std::thread::spawn(move || tool.invoke(&call).map(drop));
    while !lane.is_finished() {
        worker
            .serve_flight(relay::Patience::For(std::time::Duration::from_millis(5)))
            .unwrap();
    }
    lane.join().unwrap().unwrap();
    let rebuilt = Standing::fold(&report.ledger_dir)
        .unwrap()
        .collaboration
        .plan_holders
        .in_building(&building);
    assert_eq!(
        (worker.holders_in(&building), rebuilt.len()),
        (rebuilt, 1),
        "the live holders read the claim the gate wrote, as the restart's fold does"
    );
}

/// A claim's line is written when the model makes it, so a run whose
/// landing fails before its plan settles still owes the history the
/// line that closes it: the claim, then the node handed back
/// (sprawling-SPEC.md 8-42-8). The signal's line is the one lost here
/// because landing writes it before the plan's.
#[test]
fn a_claim_whose_landing_failed_is_handed_back() {
    use crate::assembly::fixture::*;
    let dir = city_with_plan(PLAN_ONE_FREE_ROW);
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
                "telling a neighbour",
                "tu_2",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "lab/room2",
                    "text": "the kiln row is mine",
                }),
            ),
            completion("done", None),
        ],
    );
    let worker = worker_over_faults(dir.path(), Some("signal_enqueued"));
    let mut worker = attach_provider(worker, &base_url, "m-local").unwrap();
    let landed = worker.handle(channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "take a row and tell a neighbour".to_owned(),
        goal: "one claim, one signal".to_owned(),
        mode: kernel::Mode::PlanGoal,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lost goal"),
        session: None,
        effort: None,
        model: None,
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

/// The plan's closing line reaches the ledger before the roadmap file is
/// rewritten, so a refused rewrite leaves that line on the history; the
/// claim is closed by it, and no hand-back line follows for a node the
/// history already shows closed (sprawling-SPEC.md 8-42-8).
#[test]
fn a_claim_closed_on_the_ledger_is_not_handed_back_when_the_roadmap_write_fails() {
    use crate::assembly::fixture::*;
    let dir = city_with_plan(PLAN_ONE_FREE_ROW);
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "taking a row",
                "tu_1",
                "plan",
                serde_json::json!({ "action": "claim", "node": "1" }),
            ),
            completion("done", None),
        ],
    );
    let worker = worker_over_faults(dir.path(), None);
    let mut worker = attach_provider(worker, &base_url, "m-local").unwrap();
    worker.planning.write_plan = |path, _, _| {
        Err(AxError::failure(
            AxCode::StorageFatal,
            "replace a document",
            path.display().to_string(),
        )
        .with_recovery("nothing: this writer refuses every plan"))
    };
    let landed = worker.handle(dispatch(b"refused plan"));
    drop(provider);

    assert_eq!(
        (landed.is_err(), plan_lines(&worker)),
        (
            true,
            owned(&[("roadmap_claimed", "1"), ("roadmap_blocked", "1")])
        ),
        "the claim, then the one line that closed it"
    );
}

/// A claim closes when its own closing line reaches the ledger, so a
/// ledger that takes a landing's first closing line and refuses its
/// second owes a hand-back line for the second node alone: the first
/// node's release is already the last line the history holds for it
/// (sprawling-SPEC.md 8-42-8).
#[test]
fn a_landing_refused_part_way_hands_back_only_the_nodes_it_did_not_close() {
    use crate::assembly::fixture::*;
    let dir = city_with_plan(PLAN_TWO_FREE_ROWS);
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
                "giving it back",
                "tu_2",
                "plan",
                serde_json::json!({ "action": "release", "node": "1", "reason": "not my trade" }),
            ),
            tool_completion(
                "taking the other",
                "tu_3",
                "plan",
                serde_json::json!({ "action": "claim", "node": "2" }),
            ),
            completion("done", None),
        ],
    );
    let worker = worker_over_faults(dir.path(), Some("roadmap_blocked"));
    let mut worker = attach_provider(worker, &base_url, "m-local").unwrap();
    let landed = worker.handle(dispatch(b"cut part way"));
    drop(provider);

    assert_eq!(
        (landed.is_err(), plan_lines(&worker)),
        (
            true,
            owned(&[
                ("roadmap_claimed", "1"),
                ("roadmap_claimed", "2"),
                ("roadmap_released", "1"),
                ("roadmap_released", "2"),
            ])
        ),
        "node 1 is closed by its own release; only node 2 is handed back"
    );
}

/// A run that claims a node and splits it holds nothing afterwards, and
/// the split line is the parent's fate: a landing that succeeds owes no
/// hand-back line after it, and neither the live holders nor a restart's
/// fold of the same history show the parent held (sprawling-SPEC.md
/// 8-42-8, 8-91).
#[test]
fn a_split_closes_the_claim_on_its_parent() {
    use crate::assembly::fixture::*;
    let dir = city_with_plan(PLAN_TWO_FREE_ROWS);
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
                "cutting it up",
                "tu_2",
                "plan",
                serde_json::json!({
                    "action": "split",
                    "node": "1",
                    "parts": ["run the cable", "test the element"]
                }),
            ),
            completion("done", None),
        ],
    );
    // On disk rather than over a fault layer, because the restart's fold
    // reads the ledger directory.
    let worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let mut worker = attach_provider(worker, &base_url, "m-local").unwrap();
    let landed = worker.handle(dispatch(b"split and land"));
    drop(provider);
    let building = Address::parse("lab").unwrap();
    let rebuilt = Standing::fold(&kernel::layout::CityLayout::new(dir.path()).ledger())
        .unwrap()
        .collaboration
        .plan_holders
        .in_building(&building);

    // The rebuilt fold is what guards the split today. The worker half
    // holds trivially until `RelayGate::serve` passes the claim it books
    // through `RunWorker::absorb`: the live table never sees the
    // call-time `roadmap_claimed`, so it is empty with or without a split.
    assert_eq!(
        (
            landed.is_ok(),
            plan_lines(&worker),
            rebuilt,
            worker.holders_in(&building)
        ),
        (
            true,
            owned(&[("roadmap_claimed", "1"), ("roadmap_split", "1")]),
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new()
        ),
        "the split closes node 1; nothing is handed back after it, and no fold holds it"
    );
}

/// A city whose building `lab` has `plan` as its roadmap.
fn city_with_plan(plan: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(dir.path().join("lab").join(city::ROADMAP_FILE), plan).unwrap();
    dir
}

fn dispatch(key: &[u8]) -> channels::Command {
    channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "work the plan".to_owned(),
        goal: "the plan's rows".to_owned(),
        mode: kernel::Mode::PlanGoal,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        effort: None,
        model: None,
    }
}

/// Each `roadmap_*` line on the history: its kind and the node it names.
fn plan_lines(worker: &RunWorker) -> Vec<(String, String)> {
    worker
        .ledger
        .read_raw_lines()
        .unwrap()
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|line| line["kind"].as_str().unwrap().starts_with("roadmap_"))
        .map(|line| {
            (
                line["kind"].as_str().unwrap().to_owned(),
                line["data"]["node"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn owned(lines: &[(&str, &str)]) -> Vec<(String, String)> {
    lines
        .iter()
        .map(|(kind, node)| ((*kind).to_owned(), (*node).to_owned()))
        .collect()
}
