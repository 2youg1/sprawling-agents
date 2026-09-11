// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one dispatch freezes and what it leaves behind: the handoff it
//! refuses, the job bytes the history carries, the prompt it sends, and
//! the lineage a fork of it writes.

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// A handoff that exists and cannot be read is not the absence of a
/// handoff, and the next session is assembled from it.
#[test]
fn a_handoff_that_cannot_be_read_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(building.join("room1")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n",
    );
    let room = Address::parse("lab/room1").unwrap();
    let handoff = city::handoff_path(dir.path(), &room);
    let _ = std::fs::remove_file(&handoff);
    std::fs::create_dir_all(&handoff).unwrap();

    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("nothing to do", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let outcome = worker.handle(channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "carry on".to_owned(),
        goal: "one turn".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"handoff"),
        session: None,
        effort: None,
    });

    let err = outcome.expect_err("an unreadable handoff is not an absent one");
    assert!(
        err.to_string().contains(city::HANDOFF_FILE),
        "the refusal has to name the file a person must fix: {err}"
    );
}

#[test]
fn work_offered_in_up_mode_without_a_test_does_not_land() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let note = dir.path().join("lab").join("room1").join("note.md");
    std::fs::create_dir_all(note.parent().unwrap()).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n\n`review: true`\n",
    );
    std::fs::write(&note, "before\n").unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "writing",
                "tu_1",
                "edit",
                serde_json::json!({ "path": "lab/room1/note.md", "content": "after
" }),
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
            task: "change the note".to_owned(),
            goal: "the note reads after".to_owned(),
            mode: channels::ModeTag::parse("up").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    drop(provider);

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
    assert!(
        history.contains("pr_opened"),
        "the work was offered: {history}"
    );
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "before
",
        "an improvement with no test of its own does not become the building's"
    );
}

#[test]
fn the_job_lands_in_the_room_and_the_history_carries_the_same_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let on_disk = std::fs::read_to_string(city::job_path(dir.path(), &room)).unwrap();
    assert!(on_disk.contains("measure the thing"));
    let stored = kernel::B3Hash::digest(on_disk.as_bytes()).to_string();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains(&stored),
        "the run's job locator addresses the same bytes the room holds"
    );

    // The must-read list is filled from the norms rather than recited:
    // the city's own instructions, this building's rules, and the job.
    let handoff: serde_json::Value = verified
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .find(|value| value["kind"] == "handoff_written")
        .expect("a run that freezes writes its handoff first");
    let must_read = handoff["data"]["must_read"]
        .as_array()
        .expect("the handoff carries its must-read list");
    assert_eq!(must_read.len(), 3, "city, building, job: {must_read:?}");
}

/// The prefix carries what it tells the agent to read.
///
/// Before this card the building slot held twelve bytes of address
/// and the run slot held a `cas:` hash no tool in the city can
/// resolve, so an agent was told to read its building's rules and
/// its own task and had no way to reach either.
/// The conversation an old run had has an address, and the handoff
/// names it.
///
/// The address is the room's, never the ledger's: the ledger lives under
/// the reserved subtree `read` refuses, and it is one chain for the
/// whole city. What lands beside the room is what this run's model
/// actually saw, one message per line, so `search` can find a line in
/// it and `read` can continue from that line.
#[test]
fn a_frozen_run_leaves_its_transcript_beside_the_room_and_the_handoff_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion("looking", "tu_1", "status", serde_json::json!({})),
            completion("seen", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "look around".to_owned(),
            goal: "one status call".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    drop(provider);

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let lines: Vec<serde_json::Value> = verified
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    let run = lines
        .iter()
        .find(|value| value["kind"] == "run_started")
        .map(|value| value["run"].as_str().unwrap().to_owned())
        .expect("a dispatch writes run_started");
    let handoff = lines
        .iter()
        .find(|value| value["kind"] == "handoff_written")
        .expect("a run that freezes writes its handoff");
    let expected = format!("lab/room1/{run}.jsonl");
    let context = handoff["data"]["context"].as_str().unwrap();
    assert!(
        context.contains(&format!("transcript at {expected}")),
        "the handoff carries one line giving the transcript's address: {context}"
    );
    assert!(
        !context.contains(".sprawling"),
        "the handoff never sends a reader to the ledger: {context}"
    );

    let transcript = std::fs::read_to_string(dir.path().join(&expected))
        .expect("the transcript is materialised at the address the handoff gave");
    let messages: Vec<serde_json::Value> = transcript
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message["role"] == "assistant"),
        "the model's own words are in it: {transcript}"
    );
    assert!(
        transcript.contains("tu_1"),
        "the tool call and its result are in it, as the model saw them: {transcript}"
    );
    assert!(
        std::fs::metadata(dir.path().join(&expected))
            .unwrap()
            .permissions()
            .readonly(),
        "a transcript is history: nobody edits it in place"
    );
}
