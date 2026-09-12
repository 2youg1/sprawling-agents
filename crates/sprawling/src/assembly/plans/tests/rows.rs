// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// A plan nobody could read and a plan somebody else changed are two
/// different facts, and only one of them is the person's to fix.
///
/// The distinction is not cosmetic: the claim path already refuses to
/// overwrite a row that moved, and it reports that refusal by name.
/// Reading the file as empty makes every claim look like it lost a
/// race that never happened, which sends the person to ask a resident
/// instead of to look at a file.
#[test]
fn a_plan_that_cannot_be_read_is_refused_by_name_rather_than_blamed_on_a_neighbour() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(building.join("room1")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n",
    );
    // A directory where the plan belongs. `read_to_string` then fails
    // for a reason that is not "it is not there yet" - the one reason
    // an empty plan is the right answer to - without this test having
    // to negotiate file permissions with the host.
    let plan = building.join(city::ROADMAP_FILE);
    let _ = std::fs::remove_file(&plan);
    std::fs::create_dir_all(&plan).unwrap();

    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("nothing to do", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let outcome = worker.handle(channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "claim a row".to_owned(),
        goal: "one claim".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"unreadable"),
        session: None,
        effort: None,
    });

    let err = outcome.expect_err("a plan nobody can read is not an empty plan");
    assert!(
        err.to_string().contains(city::ROADMAP_FILE),
        "the refusal has to name the file a person must fix: {err}"
    );
}

/// The other half of the rule: not only "the line comes before the
/// change", but "no line, no change".
///
/// The ledger here is the real `JsonlLedger` over the deterministic
/// power-loss model, told to lose exactly the write carrying
/// `roadmap_claimed`. Everything else in the city is on a real disk,
/// which is the point: the plan the run wanted to rewrite is a file
/// somebody could go and read afterwards.
#[test]
fn a_line_the_history_refused_is_a_change_the_city_never_made() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let plan = dir.path().join("lab").join(city::ROADMAP_FILE);
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(&plan, PLAN_TWO_FREE_ROWS).unwrap();

    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "taking a row",
                "tu_1",
                "plan",
                serde_json::json!({ "action": "claim", "node": "1" }),
            ),
            completion("took it", None),
        ],
    );

    // The one write that dies. Named by what it carries rather than
    // by an ordinal: how many filesystem operations run before it is
    // not something a caller up here knows, and any number written
    // here would stop meaning this line the moment anything upstream
    // read one more file.
    let fs = memory::FaultFs::new(memory::FaultPlan {
        cut_at_op: None,
        cut_on_write: Some("roadmap_claimed"),
        torn_tail: memory::TornTail::None,
    });
    let (ledger, _report) =
        memory::JsonlLedger::open_faulty(fs, &ledger_dir(dir.path()), now_ms().unwrap()).unwrap();
    let mut worker = RunWorker::over(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
        ledger,
    )
    .unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            tuning: channels::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 32_768,
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();

    let outcome = worker.handle(channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "take one row".to_owned(),
        goal: "one claim".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lost"),
        session: None,
        effort: None,
    });
    drop(provider);

    let after = std::fs::read_to_string(&plan).unwrap();
    assert_eq!(
        after, PLAN_TWO_FREE_ROWS,
        "the line never landed, so the plan on disk must stand exactly as it was"
    );
    assert!(
        outcome.is_err(),
        "a dispatch whose line the history refused must not report success"
    );
}

#[test]
fn a_run_takes_a_row_from_the_plan_and_the_next_run_cannot_take_the_same_one() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let plan = dir.path().join("lab").join(city::ROADMAP_FILE);
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(&plan, PLAN_TWO_FREE_ROWS).unwrap();
    let take = serde_json::json!({ "action": "claim", "node": "1" });
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion("taking a row", "tu_1", "plan", take.clone()),
            completion("took it", None),
            tool_completion("taking the same row", "tu_2", "plan", take),
            completion("took another", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for (n, room) in ["lab/room1", "lab/room2"].into_iter().enumerate() {
        worker
            .handle(channels::Command::Dispatch {
                addr: Address::parse(room).unwrap(),
                task: "take a row from the plan".to_owned(),
                goal: "claim one row, then stop".to_owned(),
                mode: channels::ModeTag::parse("plan").unwrap(),
                idem: kernel::IdemKey::derive(
                    &RunId::CITY,
                    kernel::Seq::new(u64::try_from(n).unwrap()),
                    b"dispatch",
                ),
                session: None,
                effort: None,
            })
            .unwrap();
    }
    drop(provider);

    let after = std::fs::read_to_string(&plan).unwrap();
    assert!(
        after.contains("| 1 | wire the kiln | 1 |  | In progress |  |"),
        "the plan on disk carries the claim: {after}"
    );
    assert!(
        after.contains("| 2 | glaze tests | 1 |  | Not started |  |"),
        "a refused claim rewrites nothing, not even the row it was offered: {after}"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join(
            "
",
        );
    assert_eq!(
        history.matches("roadmap_claimed").count(),
        1,
        "one row, one claim, however many runs asked for it"
    );
}

#[test]
fn a_finished_row_carries_evidence_a_reader_can_retrieve() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let plan = dir.path().join("lab").join(city::ROADMAP_FILE);
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(&plan, PLAN_ONE_FREE_ROW).unwrap();
    let evidence = format!("cas:b3-{}", "ab".repeat(32));
    // Two calls, because the plan gate admits no shortcut: a run
    // closes the node it took, and this run has to take it first.
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "taking it",
                "tu_1",
                "plan",
                serde_json::json!({ "action": "claim", "node": "1" }),
            ),
            tool_completion(
                "closing it",
                "tu_2",
                "plan",
                serde_json::json!({ "action": "finish", "node": "1", "evidence": evidence }),
            ),
            completion("closed", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "close the row".to_owned(),
            goal: "finish row 1 with evidence".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    drop(provider);

    let after = std::fs::read_to_string(&plan).unwrap();
    let kernel::RoadmapShape::WellFormed { rows } = kernel::check_roadmap_shape(&after) else {
        panic!("an edited plan still parses");
    };
    let tree = kernel::PlanTree::build(rows).expect("an edited plan is still a tree");
    let kernel::Progress::Planned(planned) = tree.progress() else {
        panic!("a plan's progress is planned")
    };
    assert_eq!(
        (planned.done, planned.total),
        (1, 1),
        "an evidenced Done is what moves the figure the person reads"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join(
            "
",
        );
    assert!(history.contains("roadmap_finished"));
}
