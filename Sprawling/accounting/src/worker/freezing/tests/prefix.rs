// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::wildcard_enum_match_arm, reason = "test code")]

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_project_that_came_with_its_own_conventions_has_them_in_the_prompt() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    std::fs::write(
        dir.path().join("lab").join("AGENTS.md"),
        "# AGENTS.md\n\nEvery measurement is recorded in millivolts.\n",
    )
    .unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("Every measurement is recorded in millivolts."),
        "the project's own conventions never reached the model: {asked}"
    );
    assert!(
        asked.contains("confidential = false"),
        "and the city's own rules for the building are still there"
    );
}

#[test]
fn the_workspaces_conventions_come_before_the_buildings_own() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    std::fs::write(
        dir.path().join("AGENTS.md"),
        "# AGENTS.md

Every commit message is in English.
",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("lab").join("AGENTS.md"),
        "# AGENTS.md

Every measurement is recorded in millivolts.
",
    )
    .unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let asked = provider.bodies().join(
        "
",
    );
    let workspace = asked.find("Every commit message is in English.");
    let building = asked.find("Every measurement is recorded in millivolts.");
    assert!(
        workspace.is_some() && building.is_some() && workspace < building,
        "the workspace's conventions and then the building's, in that order: {asked}"
    );
}

#[test]
fn a_building_without_the_file_gets_no_heading_for_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
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
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
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
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("confidential = false"),
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
fn every_segment_of_a_frozen_prompt_reads_back_as_text() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
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

    let wire::Answer::Prefix(answer) =
        crate::views::ask(dir.path(), &wire::Query::Prefix { run }).unwrap()
    else {
        panic!("Prefix answers with a prefix");
    };
    let slots: Vec<wire::PrefixSlot> = answer.segments.iter().map(|s| s.slot).collect();
    assert_eq!(
        slots,
        vec![
            wire::PrefixSlot::City,
            wire::PrefixSlot::Building,
            wire::PrefixSlot::Resident,
            wire::PrefixSlot::Run,
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
        .find(|segment| segment.slot == wire::PrefixSlot::Building)
        .expect("the building slot is one of the four");
    assert!(
        building.text.contains("confidential = false"),
        "the building's own rules read back: {}",
        building.text
    );
    assert!(
        building
            .sources
            .iter()
            .any(|source| source.addr.as_str() == "lab/.sprawling/RULES.toml"),
        "the segment names the file it was read from: {:?}",
        building.sources
    );

    let locator =
        kernel::Locator::parse(&format!("cas:b3-{}", building.hash)).expect("a stored segment");
    let wire::Answer::Content(content) =
        crate::views::ask(dir.path(), &wire::Query::Content { locator }).unwrap()
    else {
        panic!("Content answers with content");
    };
    assert_eq!(content.text, building.text);
}

fn segments_of_a_review_dispatch() -> Vec<(String, String)> {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules("review = true\n"));
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let assembled = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::PromptAssembled =>
            {
                Some(
                    record
                        .data()
                        .read::<kernel::event::record::PromptAssembled>()
                        .unwrap(),
                )
            }
            _ => None,
        })
        .expect("a dispatch assembles a prompt");
    assembled
        .segments
        .into_iter()
        .map(|segment| (segment.slot, segment.hash.to_string()))
        .collect()
}

#[test]
fn a_review_dispatch_freezes_the_prefix_it_froze_on_the_accounting_thread() {
    let pinned = [
        (
            "city",
            "abadbb252de76ade1cdc761aa6f59e69bd301f459eeee820a946b3793288e01f",
        ),
        (
            "building",
            "204ff6ab2fde14dfb3659bebae0a2e2ba4ecb5a538690b76db492ad6b3276913",
        ),
        (
            "resident",
            "8c1c559aa850afea1525b5b620e2534c03adf5a999b6542d5f77374866d8a1a0",
        ),
        (
            "run",
            "452aa91752bdd57c04a2a8165fff1de0148273fe25f9c1b87c2c4e8a78cbed57",
        ),
    ]
    .map(|(slot, hash)| (slot.to_owned(), hash.to_owned()));
    assert_eq!(segments_of_a_review_dispatch(), pinned);
}
