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
    let worker = worker_over_faults(dir.path(), Some("goal_registered"));
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
/// hand-back line after it (sprawling-SPEC.md 8-42-8).
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
    let worker = worker_over_faults(dir.path(), None);
    let mut worker = attach_provider(worker, &base_url, "m-local").unwrap();
    let landed = worker.handle(dispatch(b"split and land"));
    drop(provider);

    assert_eq!(
        (landed.is_ok(), plan_lines(&worker)),
        (
            true,
            owned(&[("roadmap_claimed", "1"), ("roadmap_split", "1")])
        ),
        "the split closes node 1; nothing is handed back after it"
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
