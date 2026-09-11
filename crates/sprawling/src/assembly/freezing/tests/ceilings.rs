// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a frozen run runs under.
//!
//! **Two spend ceilings used to be asserted here and are gone**
//!: work handed down and work an answer carried on were each
//! checked against the `BudgetCap` that sent them. There is no such
//! ceiling to carry now — nobody can price a piece of work before it
//! runs, and the one brake is `Halt`. What a run is still frozen under
//! is the effort its configuration layer states, which is the case below.

use crate::assembly::fixture::*;
use crate::assembly::*;

#[test]
fn the_effort_a_config_layer_states_is_what_goes_out_on_the_wire() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let city_layer = city::config_path(dir.path(), &room, city::Layer::City).unwrap();
    std::fs::create_dir_all(city_layer.parent().unwrap()).unwrap();
    std::fs::write(&city_layer, "[model]\neffort = \"low\"\n").unwrap();
    let building_layer = city::config_path(dir.path(), &room, city::Layer::Building).unwrap();
    std::fs::create_dir_all(building_layer.parent().unwrap()).unwrap();
    std::fs::write(&building_layer, "[model]\neffort = \"xhigh\"\n").unwrap();

    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room,
            task: "think about it".to_owned(),
            goal: "answer".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("\"effort\":\"xhigh\""),
        "the building layer overrides the city layer, and the resolved level is what the \
         provider was asked for: {asked}"
    );
    assert!(!asked.contains("\"effort\":\"low\""));
}
