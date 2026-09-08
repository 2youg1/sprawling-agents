// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// A building under review lends every run its own tree so that
/// nothing it produces is the building's until somebody else checks
/// it. This asks whether the shelf is inside that fence.
#[test]
fn a_run_under_review_puts_nothing_on_the_shelf_before_it_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(building.join("room1")).unwrap();
    std::fs::create_dir_all(building.join("room2")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n\n`review: true`\n",
    );

    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "remembering why",
                "tu_1",
                "archive",
                serde_json::json!({
                    "action": "record",
                    "kind": "decision",
                    "text": "we chose the embedded store",
                }),
            ),
            tool_completion(
                "offering",
                "tu_2",
                "pr",
                serde_json::json!({ "action": "open" }),
            ),
            completion("offered", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "decide and remember".to_owned(),
            goal: "one decision".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"remember"),
            session: None,
            effort: None,
        })
        .unwrap();

    let lab = Address::parse("lab").unwrap();
    let before = city::archive_index(dir.path(), &lab).unwrap();
    assert!(
        before.is_empty(),
        "a run under review reached the building's shelf without being checked: {before:?}"
    );

    // The other half of the same rule: fencing it must not lose it.
    // A shelf entry nobody can ever reach is a worse answer than one
    // that arrived too early.
    let branch = branch_opened(&report.ledger_dir);
    let (base_url, _second) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "checking",
                "tu_3",
                "pr",
                serde_json::json!({ "action": "check", "branch": branch, "passed": true }),
            ),
            completion("checked", None),
        ],
    );
    let mut checker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    checker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room2").unwrap(),
            task: "check the decision".to_owned(),
            goal: "one check".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"check"),
            session: None,
            effort: None,
        })
        .unwrap();

    let after = city::archive_index(dir.path(), &lab).unwrap();
    assert_eq!(
        after.len(),
        1,
        "once it was checked the decision is the building's: {after:?}"
    );
}

/// The branch a request was opened on, read back from the history.
fn branch_opened(ledger_dir: &Path) -> String {
    let verified = runtime::replay::verify_ledger_dir(ledger_dir).unwrap();
    let mut found = None;
    for line in verified.raw_lines() {
        let record = EventRecord::parse_line(line).unwrap();
        if record.kind() == EventKind::PrOpened {
            found = record
                .data()
                .as_map()
                .get("branch")
                .and_then(|v| v.as_str())
                .map(str::to_owned);
        }
    }
    found.expect("opening a request leaves a record naming the branch")
}

#[test]
fn work_in_a_review_building_reaches_it_only_after_someone_else_checks_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    // The building asks for review, so every run works in its own
    // tree and nothing lands until a second resident says so.
    let building = dir.path().join("lab");
    std::fs::create_dir_all(building.join("room1")).unwrap();
    std::fs::create_dir_all(building.join("room2")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n\n`review: true`\n",
    );
    let note = building.join("room1").join("notes.md");
    std::fs::create_dir_all(note.parent().unwrap()).unwrap();
    std::fs::write(&note, b"before\n").unwrap();
    let version = runtime::version_of(b"before\n");

    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "editing",
                "tu_1",
                "edit",
                serde_json::json!({
                    "path": "lab/room1/notes.md",
                    "base_version": version,
                    "old": "before",
                    "new": "after",
                }),
            ),
            tool_completion(
                "offering",
                "tu_2",
                "pr",
                serde_json::json!({ "action": "open" }),
            ),
            completion("offered", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "fix the notes".to_owned(),
            goal: "one edit, then offer it".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"one"),
            session: None,
            effort: None,
        })
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "before\n",
        "the implementer's work is invisible to the building until it is merged"
    );

    // A second resident checks it. The same tool, a different run.
    let branch = {
        let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
        let mut found = None;
        for line in verified.raw_lines() {
            let record = EventRecord::parse_line(line).unwrap();
            if record.kind() == EventKind::PrOpened {
                found = record
                    .data()
                    .as_map()
                    .get("branch")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned);
            }
        }
        found.expect("opening a request leaves a record naming the branch")
    };
    let (base_url, _second) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "checking",
                "tu_3",
                "pr",
                serde_json::json!({ "action": "check", "branch": branch, "passed": true }),
            ),
            completion("checked", None),
        ],
    );
    let mut checker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    checker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room2").unwrap(),
            task: "check the notes".to_owned(),
            goal: "one check, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"two"),
            session: None,
            effort: None,
        })
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "after\n",
        "once someone else has checked it, the building stands on the work"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("worktree_opened"));
    assert!(history.contains("pr_merged"));
}
