// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// The other half of the same rule: a steer-kind signal slips under
/// the door of the run it reaches, landing at that run's next safe
/// point with the sender's address in front of it. `collab::steer`
/// has held both entrances since P2.04 and the resident's one had no
/// caller until now.
#[test]
fn a_steer_from_a_resident_lands_in_the_window_as_that_resident() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("market").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    for who in ["ito", "hana"] {
        let room = dir.path().join("market").join(who);
        std::fs::create_dir_all(&room).unwrap();
        std::fs::write(
            room.join(city::URBANITE_FILE),
            format!("# URBANITE.md\n\nTrades in the market as {who}.\n"),
        )
        .unwrap();
    }

    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "cutting in",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/hana",
                    "kind": "steer",
                    "text": "drop the glaze order, the kiln comes first",
                }),
            ),
            completion("sent", None),
            tool_completion("where do I stand", "tu_2", "status", serde_json::json!({})),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "tell hana what matters first".to_owned(),
            goal: "hana knows".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("@market/ito: drop the glaze order"),
        "the steer lands at the end of a tool result, attributed to the resident who sent it"
    );
    assert!(
        !asked.contains("user: drop the glaze order"),
        "only the person's entrance can render as the person"
    );
}

/// A dispatch the city will not take leaves nothing a person can
/// find.
///
/// `dispatch_in` opens by saying so - a halted city that laid a job
/// file down would leave a task in a room no run ever opened - and
/// the halt was the only door that held it. The room was opened and
/// the brief was written before the tag was resolved, so a dispatch
/// to a building nobody raised was refused for having no model and
/// left a building, a room and a `JOB.md` behind: files a person can
/// see that no record in the one history mentions.
///
/// The code is asserted beside the disk because the two are separate
/// promises. The caller is owed `E_CONFIG_INVALID` here - nothing is
/// attached, so no tag names a model - and moving the judgement in
/// front of the write must not change which refusal that is.
#[test]
fn a_dispatch_the_city_will_not_take_leaves_no_room_behind() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let refused = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("gamma").unwrap(),
            task: "say something".to_owned(),
            goal: "an answer".to_owned(),
            mode: channels::ModeTag::parse("build").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: Some(kernel::SessionName::parse("one").unwrap()),
            effort: None,
        })
        .unwrap_err();

    assert_eq!(
        *refused.code(),
        AxCode::ConfigInvalid,
        "a city with nothing attached refuses a dispatch for having no model: {refused}"
    );
    assert!(
        !dir.path().join("gamma").exists(),
        "a refused dispatch raised a building nobody asked for"
    );
}

/// A dispatch that never says when to stop is a conversation, and the
/// prefix says so instead of handing over a form with its one
/// irreplaceable field blank.
#[test]
fn a_dispatch_with_no_goal_leaves_no_job_file_and_says_the_person_is_here() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "what do you make of this".to_owned(),
            goal: String::new(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"talk"),
            session: None,
            effort: None,
        })
        .unwrap();

    assert!(
        !city::job_path(dir.path(), &room).exists(),
        "a conversation writes no task file"
    );
    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("working with the person directly"),
        "the prefix states the situation: {asked}"
    );
    assert!(
        !asked.contains("Task: what do you make of this"),
        "the person's line goes out as they wrote it, not as a form"
    );
}
