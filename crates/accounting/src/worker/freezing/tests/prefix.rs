// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the frozen prefix carries: the building's rules, the project's
//! own conventions, and the task - all as bytes rather than as pointers
//! at bytes.

#![allow(clippy::wildcard_enum_match_arm, reason = "test code")]

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

/// A building that was adopted rather than raised brings its own
/// conventions with it, written for whoever works on that project, and
/// `AGENTS.md` is the file they are written in. A resident that has to
/// open it before it can follow it follows it one turn late, or not at
/// all - the same sentence `building_segment` already carries about
/// `RULES.toml`, and the same answer.
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
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
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

/// A city formed around a project's own folder finds the project's
/// `AGENTS.md` at the city root, and its buildings are that project's
/// subfolders. Both files reach the prompt, the workspace's first and
/// the building's after it, so the one nearest the work is read last.
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

/// A building with no `AGENTS.md` says nothing about one. The heading is
/// written only when there is a file under it, so a resident is never
/// told to follow conventions that do not exist.
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
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "measure the thing".to_owned(),
            goal: "a number with a unit, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
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

/// Every one of the four segments is interned: a hash on the ledger with
/// nothing holding its bytes would leave "what was this agent told"
/// without an answer. `Query::Prefix` is the read that proves it.
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

    // And the general store read reaches the same bytes by locator.
    let locator =
        kernel::Locator::parse(&format!("cas:b3-{}", building.hash)).expect("a stored segment");
    let wire::Answer::Content(content) =
        crate::views::ask(dir.path(), &wire::Query::Content { locator }).unwrap()
    else {
        panic!("Content answers with content");
    };
    assert_eq!(content.text, building.text);
}

/// Dispatches one fixed job into a room under review in a new city and
/// reads back the segments its prompt was assembled from, as
/// `(slot, hash)`.
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

/// Preparing a dispatch moves off the accounting thread without moving
/// a byte of the prefix it freezes (`crates/sprawling/Spec.lean` §8-113): the hashes
/// below were taken from the path that prepares everything on the
/// accounting thread, and a room under review is where the move reaches
/// furthest, because its tree is placed in the lane. A change that means
/// to alter what a fresh city's prefix says takes the new hashes from
/// this test's failure; a change that only moves work between threads
/// leaves them as they are.
#[test]
fn a_review_dispatch_freezes_the_prefix_it_froze_on_the_accounting_thread() {
    let pinned = [
        (
            "city",
            "868c7c3df7596cdf7edcfb6fa2514b5df484969d8d814c9160749bddbe8aa269",
        ),
        (
            "building",
            "955c58cf462564ab1f9a3194306e53ac7ccc65c2aa8c4ee847b8a15b6d4b48bd",
        ),
        (
            "resident",
            "ffd2c481526130d470244e7e931265348af859b872f505fd186e3505337f4ade",
        ),
        (
            "run",
            "cd3fafe6e3ac5d33658800023c6017901231907794a69a2ac3a176d4ca416134",
        ),
    ]
    .map(|(slot, hash)| (slot.to_owned(), hash.to_owned()));
    assert_eq!(segments_of_a_review_dispatch(), pinned);
}
