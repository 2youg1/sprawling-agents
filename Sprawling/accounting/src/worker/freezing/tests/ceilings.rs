// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn the_effort_a_config_layer_states_is_what_goes_out_on_the_wire() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let city_layer = city::config_path(dir.path(), &room, city::Layer::City).unwrap();
    std::fs::create_dir_all(city_layer.parent().unwrap()).unwrap();
    std::fs::write(&city_layer, "[model]\neffort = \"low\"\n").unwrap();
    let building_layer = city::config_path(dir.path(), &room, city::Layer::Building).unwrap();
    std::fs::create_dir_all(building_layer.parent().unwrap()).unwrap();
    std::fs::write(&building_layer, "[model]\neffort = \"xhigh\"\n").unwrap();

    let (base_url, provider) = fake_openai(&["m-thinks"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-thinks").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: room,
            task: "think about it".to_owned(),
            goal: "answer".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("\"reasoning_effort\":\"xhigh\""),
        "the building layer overrides the city layer, and the resolved level is what the \
         provider was asked for: {asked}"
    );
    assert!(!asked.contains("\"reasoning_effort\":\"low\""));
}
