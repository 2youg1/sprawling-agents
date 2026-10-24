// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two layers a building may override the city's prompt at: its own
//! `City.md` and its own note for a model, each in the building's
//! reserved subtree, each replacing the city's rather than joining it.

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

const CITY_NOTE: &str = "The city's note: answer in one paragraph.";
const BUILDING_NOTE: &str = "The lab's note: answer with a table.";
const BUILDING_PROMPT: &str = "# City.md - the lab\n\nThe lab speaks only of voltages.\n";

/// What one dispatch to `room` sent as its system prompt, in a city with
/// the buildings `lab` and `shop`, where `lay` has written whatever
/// overrides the case needs.
fn system_prompt_at(room: &str, lay: impl FnOnce(&std::path::Path)) -> String {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for building in ["lab", "shop"] {
        worker
            .handle(wire::Command::CreateBuilding {
                addr: Address::parse(building).unwrap(),
                template: wire::TemplateName::parse("minimal").unwrap(),
                idem: kernel::IdemKey::derive(
                    &RunId::CITY,
                    kernel::Seq::FIRST,
                    building.as_bytes(),
                ),
            })
            .unwrap();
    }
    lay(dir.path());
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse(room).unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let body: serde_json::Value = provider
        .bodies()
        .iter()
        .find_map(|body| serde_json::from_str(body).ok())
        .unwrap();
    body["messages"][0]["content"].as_str().unwrap().to_owned()
}

fn write(path: std::path::PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn lab_prompt(root: &std::path::Path) {
    write(
        root.join("lab")
            .join(kernel::RESERVED_PREFIX)
            .join("City.md"),
        BUILDING_PROMPT,
    );
}

fn both_notes(root: &std::path::Path) {
    let note = |base: std::path::PathBuf| base.join("models").join("house").join("m-local.md");
    write(note(root.join(kernel::RESERVED_PREFIX)), CITY_NOTE);
    write(
        note(root.join("lab").join(kernel::RESERVED_PREFIX)),
        BUILDING_NOTE,
    );
}

/// A building's own `City.md` is what its runs are told, and the city's
/// is not; a building without one is told the city's.
#[test]
fn a_buildings_city_md_replaces_the_citys_there_and_only_there() {
    let lab = system_prompt_at("lab/room1", lab_prompt);
    assert!(lab.starts_with(BUILDING_PROMPT), "{lab}");
    assert!(!lab.contains(city::Shipped::City.text()), "{lab}");

    let shop = system_prompt_at("shop/room1", lab_prompt);
    assert!(shop.starts_with(city::Shipped::City.text()), "{shop}");
    assert!(!shop.contains("The lab speaks only of voltages."), "{shop}");
}

/// A building's own note for a model ends the prompt in its place of the
/// city's; elsewhere the city's note stands.
#[test]
fn a_buildings_note_for_a_model_replaces_the_citys_there_and_only_there() {
    let lab = system_prompt_at("lab/room1", both_notes);
    assert!(lab.ends_with(&format!("\n\n{BUILDING_NOTE}")), "{lab}");
    assert!(!lab.contains(CITY_NOTE), "{lab}");

    let shop = system_prompt_at("shop/room1", both_notes);
    assert!(shop.ends_with(&format!("\n\n{CITY_NOTE}")), "{shop}");
}
