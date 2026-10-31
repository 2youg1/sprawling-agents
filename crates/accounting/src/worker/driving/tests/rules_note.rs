// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The project's rules below a run's own room: handed over once, on the
//! `read` that met them, and never a second time
//! (`crates/runtime/spec/Tools/Read.lean` §8-29-7).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::super::*;
use crate::worker::fixture::*;

#[test]
fn a_rule_deeper_than_the_run_s_room_arrives_once_on_the_read_that_met_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "reading the probe",
                "read",
                "tu_1",
                serde_json::json!({ "file_path": "lab/pkg/probe/probe.rs" }),
            ),
            completion_with(
                "reading it again",
                "read",
                "tu_2",
                serde_json::json!({ "file_path": "lab/pkg/probe/probe.rs" }),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let lab = dir.path().join("lab");
    std::fs::create_dir_all(lab.join("pkg").join("probe")).unwrap();
    std::fs::write(lab.join("AGENTS.md"), "Measurements are in millivolts.\n").unwrap();
    std::fs::write(
        lab.join("pkg").join("probe").join("AGENTS.md"),
        "The probe samples at one hertz.\n",
    )
    .unwrap();
    std::fs::write(
        lab.join("pkg").join("probe").join("probe.rs"),
        "pub fn probe() -> u32 { 1 }\n",
    )
    .unwrap();

    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/pkg").unwrap(),
            task: "read the probe".to_owned(),
            goal: "the number it returns".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    // The listing probe answers no body, so only the chat requests are
    // counted here.
    let bodies: Vec<String> = provider
        .bodies()
        .into_iter()
        .filter(|body| !body.is_empty())
        .collect();
    assert_eq!(bodies.len(), 3, "three turns were scripted");
    let sentence = "The probe samples at one hertz.";
    assert!(
        !bodies[0].contains(sentence),
        "the frozen prefix carries the rules of the building and the run's own room, \
         and this one is deeper"
    );
    assert_eq!(
        bodies[1].matches(sentence).count(),
        1,
        "the read that opened the file carried the rule once: {}",
        bodies[1]
    );
    assert_eq!(
        bodies[2].matches(sentence).count(),
        1,
        "the second read of the same file is not given the rule again: {}",
        bodies[2]
    );
    // The prefix and the tool table are the same bytes in every request:
    // the rule arrived at the end of the conversation, not in either of
    // them, so nothing before the first message moved.
    let parsed: Vec<serde_json::Value> = bodies
        .iter()
        .map(|body| serde_json::from_str(body).unwrap())
        .collect();
    assert_eq!(
        parsed[0]["tools"], parsed[2]["tools"],
        "the tool table did not move"
    );
    assert_eq!(
        parsed[0]["messages"][0]["content"], parsed[2]["messages"][0]["content"],
        "the system block did not move"
    );
    assert!(
        bodies[1].contains("lab/pkg/probe/AGENTS.md"),
        "the note says which file the words came from"
    );
}
