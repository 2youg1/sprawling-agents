// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The note a person keeps for one model: sent at the end of the system
//! prompt to that model, and to no other.

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

const NOTE: &str = "Quote the failing line of every test run in full.";

/// A city with one building, a note for `house/<noted>`, and a worker
/// whose main model is `m-local` on the `house` endpoint; returns the
/// system text of the first request the dispatch sent.
fn system_prompt_with_a_note_for(noted: &str) -> String {
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
    let notes = dir
        .path()
        .join(kernel::RESERVED_PREFIX)
        .join("models")
        .join("house");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join(format!("{noted}.md")), NOTE).unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number, then stop".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    // The first chat request: the endpoint also answers a model listing,
    // which carries no body.
    let body: serde_json::Value = provider
        .bodies()
        .iter()
        .find_map(|body| serde_json::from_str(body).ok())
        .unwrap();
    assert_eq!(body["messages"][0]["role"], "system");
    body["messages"][0]["content"].as_str().unwrap().to_owned()
}

#[test]
fn a_note_for_the_chosen_model_ends_the_system_prompt() {
    let system = system_prompt_with_a_note_for("m-local");
    assert!(
        system.ends_with(&format!("\n\n{NOTE}")),
        "the system prompt does not end with the model's note: {system}"
    );
}

#[test]
fn a_note_for_another_model_is_not_sent() {
    let system = system_prompt_with_a_note_for("m-remote");
    assert!(
        !system.contains(NOTE),
        "another model's note reached this one: {system}"
    );
}
