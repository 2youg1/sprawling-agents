// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the frozen prefix carries: the building's rules, the project's
//! own conventions, and the task - all as bytes rather than as pointers
//! at bytes.

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// A building that was adopted rather than raised brings its own
/// conventions with it, written for whoever works on that project, and
/// `AGENTS.md` is the file they are written in. A resident that has to
/// open it before it can follow it follows it one turn late, or not at
/// all - the same sentence `building_segment` already carries about
/// `BUILDING.md`, and the same answer.
#[test]
fn a_project_that_came_with_its_own_conventions_has_them_in_the_prompt() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    // Where the project keeps it: the building's own root, outside the
    // reserved subtree, because it belongs to the project rather than
    // to the city.
    std::fs::write(
        dir.path().join("lab").join("AGENTS.md"),
        "# AGENTS.md\n\nEvery measurement is recorded in millivolts.\n",
    )
    .unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("Every measurement is recorded in millivolts."),
        "the project's own conventions never reached the model: {asked}"
    );
    assert!(
        asked.contains("confidential: false"),
        "and the city's own rules for the building are still there"
    );
}

/// A building with no `AGENTS.md` says nothing about one. The heading is
/// written only when there is a file under it, so a resident is never
/// told to follow conventions that do not exist.
#[test]
fn a_building_without_the_file_gets_no_heading_for_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
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
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    assert!(
        !provider.bodies().join("\n").contains("AGENTS.md"),
        "a file that is not there is not announced"
    );
}

#[test]
fn the_prefix_carries_the_rules_and_the_task_rather_than_pointing_at_them() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    // A handoff the last session actually wrote, as against the blank
    // form a new room starts with. It is the room's, not the
    // building's.
    let handoff = city::handoff_path(dir.path(), &room);
    if let Some(parent) = handoff.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(
        &handoff,
        "# Handoff \u{2014} lab/room1\n\nThe meter reads in millivolts.\n",
    )
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

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("confidential: false"),
        "the building's own rules reach the model: {asked}"
    );
    assert!(
        asked.contains("The meter reads in millivolts."),
        "what the last session left reaches the next one"
    );
    assert!(
        asked.contains("a number with a unit, then stop"),
        "this session's brief is in the prompt, not addressed by it"
    );
    assert!(
        !asked.contains("FULL READ"),
        "nothing sends the agent after what it already holds"
    );
    assert!(
        !asked.contains("cas:b3-"),
        "no content hash reaches a model that cannot resolve one"
    );
}

#[test]
fn a_fork_records_lineage_and_refuses_a_node_the_mother_does_not_own() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "mother work".to_owned(),
            goal: "a lineage".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    // Find the mother's run_started node in the verified chain.
    let verified = runtime::replay::verify_ledger_dir(&ledger_dir(dir.path())).unwrap();
    let (mother, node) = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunStarted =>
            {
                Some((record.run(), record.seq()))
            }
            _ => None,
        })
        .expect("a dispatch writes run_started");
    let new_run = worker.fork(mother, node, None).unwrap();
    assert_ne!(new_run, mother);
    let after = runtime::replay::verify_ledger_dir(&ledger_dir(dir.path())).unwrap();
    let forked = after
        .lines()
        .iter()
        .filter_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunForked =>
            {
                Some(record.clone())
            }
            _ => None,
        })
        .next()
        .expect("the fork is a ledger fact");
    assert_eq!(forked.run(), new_run);
    // Seq 0 is the genesis, a city event: not the mother's node.
    let err = worker.fork(mother, kernel::Seq::FIRST, None).unwrap_err();
    assert!(err.subject().contains("not an event of run"), "{err}");
}

/// Two of the four segments used to be nowhere but the prompt: their
/// hashes were on the ledger and nothing held the bytes, so "what was
/// this agent told" had no answer. Every segment is interned now, and
/// `Query::Prefix` is the read that proves it.
#[test]
fn every_segment_of_a_frozen_prompt_reads_back_as_text() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
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
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&ledger_dir(dir.path())).unwrap();
    let run = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::PromptAssembled =>
            {
                Some(record.run())
            }
            _ => None,
        })
        .expect("a dispatch assembles a prompt");

    let channels::Answer::Prefix(answer) =
        crate::views::ask(dir.path(), &channels::Query::Prefix { run }).unwrap()
    else {
        panic!("Prefix answers with a prefix");
    };
    let slots: Vec<channels::PrefixSlot> = answer.segments.iter().map(|s| s.slot).collect();
    assert_eq!(
        slots,
        vec![
            channels::PrefixSlot::City,
            channels::PrefixSlot::Building,
            channels::PrefixSlot::Resident,
            channels::PrefixSlot::Run,
        ]
    );
    for segment in &answer.segments {
        assert!(segment.stored, "{:?} is not in the store", segment.slot);
        assert_eq!(
            u64::try_from(segment.text.len()).unwrap(),
            segment.bytes,
            "{:?} reads back shorter than it was frozen",
            segment.slot
        );
    }
    let building = answer
        .segments
        .iter()
        .find(|segment| segment.slot == channels::PrefixSlot::Building)
        .expect("the building slot is one of the four");
    assert!(
        building.text.contains("confidential: false"),
        "the building's own rules read back: {}",
        building.text
    );
    assert!(
        building
            .sources
            .iter()
            .any(|source| source.addr.as_str() == "lab/.sprawling/BUILDING.md"),
        "the segment names the file it was read from: {:?}",
        building.sources
    );

    // And the general store read reaches the same bytes by locator.
    let locator =
        kernel::Locator::parse(&format!("cas:b3-{}", building.hash)).expect("a stored segment");
    let channels::Answer::Content(content) =
        crate::views::ask(dir.path(), &channels::Query::Content { locator }).unwrap()
    else {
        panic!("Content answers with content");
    };
    assert_eq!(content.text, building.text);
}
