// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[cfg(feature = "sandbox")]
use super::engine::execution_engine;
use super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

#[test]
fn a_building_whose_rules_do_not_parse_stops_the_run_rather_than_guessing() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "lab", "# BUILDING.md\n\nnothing declared\n");
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let err = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "anything".to_owned(),
            goal: "anything".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap_err();
    assert!(err.recovery().contains("confidential: false"));
}

/// Until this tool existed a run could only signal an address
/// somebody had already handed it, and a guessed one opened a queue
/// nobody read. The evidence has to come through the production
/// path, because what is being claimed is that the model is *told*.
#[test]
fn a_run_is_told_who_shares_its_building_and_what_to_bring_them() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    let mason = dir.path().join("lab").join("mason");
    std::fs::create_dir_all(&mason).unwrap();
    std::fs::write(
        mason.join(city::URBANITE_FILE),
        "# URBANITE.md \u{2014} mason\n\n## Bring them\n\nAnything that has to survive a firing.\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("store")).unwrap();

    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion("who is here", "tu_1", "neighbours", serde_json::json!({})),
            tool_completion(
                "and where do I stand",
                "tu_2",
                "status",
                serde_json::json!({}),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "find out who else is here".to_owned(),
            goal: "one answer is enough".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    assert!(
        provider.bodies().join("\n").contains("neighbours"),
        "the tool table the model is given carries the way to ask"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("Anything that has to survive a firing"),
        "the line a resident wrote about itself is what reaches the one asking"
    );
    assert!(
        history.contains("lab/store"),
        "an open room is a place to send work to, not something to hide"
    );
    assert!(
        history.contains("neighbours: 1"),
        "status counts people rather than places: two rooms, one resident"
    );
}

#[test]
fn a_signal_one_run_sends_is_read_by_the_run_that_pulls_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "telling room2",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "lab/room2",
                    "kind": "mention",
                    "text": "the kiln is free after four",
                }),
            ),
            completion("told them", None),
            tool_completion(
                "checking",
                "tu_2",
                "signal",
                serde_json::json!({ "action": "pull" }),
            ),
            completion("read it", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for (n, room) in ["lab/room1", "lab/room2"].into_iter().enumerate() {
        worker
            .handle(channels::Command::Dispatch {
                addr: Address::parse(room).unwrap(),
                task: "talk to the neighbour".to_owned(),
                goal: "one message, then stop".to_owned(),
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

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("signal_enqueued"),
        "a signal a tool sent is a fact the history keeps"
    );
    assert!(
        history.contains("signal_consumed"),
        "and taking it is a second fact, written by whoever took it"
    );

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("the kiln is free after four"),
        "the point of the mechanism is that the other resident reads it"
    );
}

/// A build that says it carries an execution engine carries one.
///
/// `AbsentSandbox` refuses with `this build carries no execution
/// engine` and tells the reader to install a build with the `wasm`
/// feature. Until the feature and this selection existed there was
/// no such build: the absent engine was written into `dispatch_in`
/// as a literal, so the sentence named an action nobody could take
/// and `runtime::WasmtimeSandbox` had no caller outside its own
/// tests.
#[cfg(feature = "sandbox")]
#[test]
fn a_build_with_the_engine_feature_carries_one() {
    let mut engine = execution_engine().expect("a build with the feature starts its engine");
    // A module that is not there: whatever this reports, it is the
    // engine reporting it rather than the absence of one.
    let job = runtime::SandboxJob {
        wasm: std::path::PathBuf::from("no-such-module.wasm"),
        argv: Vec::new(),
        env: Vec::new(),
        stdin: Vec::new(),
        mounts: Vec::new(),
        fuel: runtime::Fuel(1_000),
    };
    let said = format!("{:?}", engine.run(&job));
    assert!(
        !said.contains("this build carries no execution engine"),
        "the feature is on and the run still met the absent engine: {said}"
    );
}

#[test]
fn the_shell_arm_exists_only_where_a_layer_asked_for_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let closed = city::load_config(dir.path(), &room).unwrap();
    assert!(
        !closed.sandbox.shell,
        "silence is the closed answer, on every layer"
    );
    let building = city::config_path(dir.path(), &room, city::Layer::Building).unwrap();
    std::fs::create_dir_all(building.parent().unwrap()).unwrap();
    std::fs::write(&building, "[sandbox]\nshell = true\nfuel = 1000\n").unwrap();
    let opened = city::load_config(dir.path(), &room).unwrap();
    assert!(opened.sandbox.shell);
    assert_eq!(opened.sandbox.fuel, 1000);
}

/// The Mayor writes Markdown and plans; it does not build. What holds
/// that is the tool table the run is given, not the wording of
/// `MAYOR.md`: an invariant a prompt is asked to keep is not one.
///
/// The evidence comes through the production path, because what is
/// being claimed is about the bytes the provider receives.
#[test]
fn a_resident_of_the_hall_is_given_no_way_to_build() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("noted", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse(kernel::consts_policy::HALL_MAYOR).unwrap(),
            task: "plan the east wing".to_owned(),
            goal: "one plan is enough".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let offered = provider.bodies().join("\n");
    for absent in ["\"exec\"", "\"delegate\"", "\"workshop\""] {
        assert!(
            !offered.contains(absent),
            "the hall was offered {absent}; the Mayor plans, it does not build"
        );
    }
    assert!(
        offered.contains("\"city\""),
        "raising a building is the one thing the hall is for, and it needs a door to it"
    );
    assert!(
        offered.contains("\"plan\""),
        "and the plan tool, which is what the hall writes"
    );
}
