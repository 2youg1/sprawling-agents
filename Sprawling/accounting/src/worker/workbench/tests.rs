// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_building_whose_rules_do_not_parse_stops_the_run_rather_than_guessing() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "lab", "nothing declared\n");
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let err = worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "anything".to_owned(),
            goal: "anything".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap_err();
    assert!(err.recovery().contains(city::RULES_FILE));
}

#[test]
fn a_run_is_told_who_shares_its_building_and_what_to_bring_them() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "find out who else is here".to_owned(),
            goal: "one answer is enough".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
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
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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
            .handle(wire::Command::Dispatch {
                addr: Address::parse(room).unwrap(),
                task: "talk to the neighbour".to_owned(),
                goal: "one message, then stop".to_owned(),
                policy: kernel::RunPolicy::of(kernel::Mode::Work),
                idem: kernel::IdemKey::derive(
                    &RunId::CITY,
                    kernel::Seq::new(u64::try_from(n).unwrap()),
                    b"dispatch",
                ),
                session: None,
                effort: None,
                model: None,
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

#[test]
fn the_shell_arm_exists_only_where_a_layer_asked_for_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
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

#[test]
fn a_resident_of_the_hall_is_given_no_way_to_build() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();

    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "",
                runtime::ToolTool::NAME,
                "tu_1",
                serde_json::json!({ "op": "list", "namespace": "sprawling" }),
            ),
            completion("noted", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse(kernel::consts_policy::HALL_MAYOR).unwrap(),
            task: "plan the east wing".to_owned(),
            goal: "one plan is enough".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    let offered = provider.bodies().join("\n");
    for absent in ["\"exec\"", "\"delegate\"", "\"workshop\""] {
        assert!(
            !offered.contains(absent),
            "the hall was offered {absent}; the Mayor plans, it does not build"
        );
    }

    let listed = |name: &str| {
        offered.contains(&format!("- {name}:")) || offered.contains(&format!("- {name}\\n"))
    };
    assert!(
        listed("city"),
        "raising a building is the one thing the hall is for, and it needs a door to it"
    );
    assert!(
        listed("plan"),
        "and the plan tool, which is what the hall writes"
    );
    assert!(offered.contains("\"tool\""));
}

#[test]
fn a_checkpoint_carries_what_the_run_may_write_and_not_only_its_room() {
    let dir = tempfile::tempdir().unwrap();
    let repo = a_note_beside_the_hall(dir.path());

    assert!(
        dir.path().join("hall").join("note.md").exists(),
        "the write has to land before a checkpoint can be asked to carry it"
    );
    let note = std::path::Path::new("hall/note.md");
    let carried: Vec<String> = repo
        .references()
        .unwrap()
        .flatten()
        .filter_map(|reference| {
            let oid = reference.target()?;
            let tree = repo.find_commit(oid).ok()?.tree().ok()?;
            tree.get_path(note).ok()?;
            Some(reference.name().unwrap_or("?").to_owned())
        })
        .collect();
    assert!(
        !carried.is_empty(),
        "no commit in this repository carries hall/note.md, so the city \
         wrote a file it can neither show in a diff nor restore"
    );
}

#[test]
fn the_checkpoint_after_an_edit_stages_the_edited_path_and_not_the_domain() {
    let dir = tempfile::tempdir().unwrap();
    let repo = a_note_beside_the_hall(dir.path());
    let mut subjects: Vec<String> = repo
        .references_glob("refs/sprawling/runs/*")
        .unwrap()
        .flatten()
        .filter_map(|reference| {
            let commit = repo.find_commit(reference.target()?).ok()?;
            Some(commit.summary().ok()??.to_owned())
        })
        .collect();
    subjects.sort();
    assert_eq!(
        subjects,
        vec![
            "checkpoint: hall".to_owned(),
            "checkpoint: hall/note.md".to_owned()
        ]
    );
}

fn a_note_beside_the_hall(city: &std::path::Path) -> git2::Repository {
    crate::worker::fixture::init_city(city).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("writing the note", Some(("c1", "hall/note.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(city, &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse(kernel::consts_policy::HALL_MAYOR).unwrap(),
            task: "leave a note beside the hall".to_owned(),
            goal: "one note".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    git2::Repository::open(city).unwrap()
}

#[test]
fn the_citys_genesis_hash_is_read_once_and_survives_the_ledger_going_away() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    let first = worker.city_hash().unwrap();

    std::fs::remove_dir_all(&report.ledger_dir).unwrap();
    assert_eq!(worker.city_hash().unwrap(), first);
}
