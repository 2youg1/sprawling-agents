// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

const NOTE: &str = "Quote the failing line of every test run in full.";

fn system_prompt_with_a_note_for(noted: &str) -> String {
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
    let notes = dir
        .path()
        .join(kernel::RESERVED_PREFIX)
        .join("models")
        .join("house");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join(format!("{noted}.md")), NOTE).unwrap();
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
