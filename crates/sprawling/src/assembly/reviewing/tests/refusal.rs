// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// The merge is the last change in the city that still outran the
/// line announcing it. `trees.merge` moved the trunk and only then
/// wrote `pr_merged`, so a refused line left the building standing on
/// work its own history says was never brought in.
#[test]
fn a_merge_the_history_refused_leaves_the_building_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(building.join("room1")).unwrap();
    std::fs::create_dir_all(building.join("room2")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md

`confidential: false`

`review: true`
",
    );
    let note = building.join("room1").join("notes.md");
    std::fs::write(
        &note, b"before
",
    )
    .unwrap();
    let version = runtime::version_of(
        b"before
",
    );

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

    // The checking run works over a ledger that will lose exactly the
    // line saying the work was merged.
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
    let fs = memory::FaultFs::new(memory::FaultPlan {
        cut_at_op: None,
        cut_on_write: Some("pr_merged"),
        torn_tail: memory::TornTail::None,
    });
    let (ledger, _report) =
        memory::JsonlLedger::open_faulty(fs, &ledger_dir(dir.path()), now_ms().unwrap()).unwrap();
    let mut checker = RunWorker::over(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
        ledger,
    )
    .unwrap();
    checker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    checker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 32_768,
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    let outcome = checker.handle(channels::Command::Dispatch {
        addr: Address::parse("lab/room2").unwrap(),
        task: "check the notes".to_owned(),
        goal: "one check, then stop".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"two"),
        session: None,
        effort: None,
    });

    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "before
",
        "the line saying the work was merged never landed, so the building must still stand              on what it had"
    );
    assert!(
        outcome.is_err(),
        "a merge whose line the history refused must not report success"
    );
}
