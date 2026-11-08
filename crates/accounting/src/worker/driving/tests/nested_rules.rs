// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn deeper_rules_arrive_once_while_the_system_and_tool_bytes_stay_frozen() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    for (path, text) in [
        ("lab/nested/pkg/AGENTS.md", "package rules\n"),
        ("lab/nested/pkg/probe/AGENTS.md", "probe rules\n"),
        ("lab/nested/pkg/probe/a.rs", "pub fn a() {}\n"),
        ("lab/nested/pkg/probe/b.rs", "pub fn b() {}\n"),
    ] {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "read a",
                "read",
                "tu_a",
                serde_json::json!({"file_path":"lab/nested/pkg/probe/a.rs"}),
            ),
            completion_with(
                "read b",
                "read",
                "tu_b",
                serde_json::json!({"file_path":"lab/nested/pkg/probe/b.rs"}),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/nested").unwrap(),
            task: "read two files".to_owned(),
            goal: "see each rule once".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    let bodies: Vec<serde_json::Value> = provider
        .bodies()
        .iter()
        .filter_map(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .filter(|body| body.get("messages").is_some())
        .collect();
    assert_eq!(bodies.len(), 3, "three scripted turns: {bodies:?}");
    for body in &bodies[1..] {
        assert_eq!(body["system"], bodies[0]["system"]);
        assert_eq!(body["tools"], bodies[0]["tools"]);
    }
    for heading in [
        "## lab/nested/pkg/AGENTS.md",
        "## lab/nested/pkg/probe/AGENTS.md",
    ] {
        assert_eq!(
            bodies[1].to_string().matches(heading).count(),
            1,
            "{}",
            bodies[1]
        );
        assert_eq!(
            bodies[2].to_string().matches(heading).count(),
            1,
            "a second file in the same room must not deliver another copy: {}",
            bodies[2]
        );
    }
    assert!(bodies[2].to_string().contains("pub fn b()"));
}
