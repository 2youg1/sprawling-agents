// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! What `delegate` does the moment it is called (collab D7): the child
//! starts while its parent still works.

use super::super::*;
use super::tests::move_in;
use crate::worker::fixture::*;

/// The provider holds the parent's second call until a third call
/// arrives, so the child's first call has to come from a run started
/// while the parent drives: what is measured is where the child is
/// started, not which lane wins a race.
fn holding_the_second_call_for_a_third() -> Pace {
    let calls = std::sync::Arc::new((std::sync::Mutex::new(0u32), std::sync::Condvar::new()));
    std::sync::Arc::new(move |request: &str| {
        if !request.starts_with("POST ") {
            return;
        }
        let (count, arrived) = &*calls;
        let mut count = count.lock().unwrap();
        *count += 1;
        arrived.notify_all();
        if *count == 2 {
            drop(
                arrived
                    .wait_timeout_while(count, std::time::Duration::from_secs(10), |seen| *seen < 3)
                    .unwrap(),
            );
        }
    })
}

fn position_of(lines: &[serde_json::Value], kind: &str, room: &str) -> Option<usize> {
    lines
        .iter()
        .position(|line| line["kind"] == kind && line.to_string().contains(&format!("\"{room}\"")))
}

#[test]
fn a_delegated_child_starts_before_its_parent_freezes() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "lab/lead");
    let (base_url, _provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![
            tool_completion(
                "handing it down",
                "tu_1",
                "delegate",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("handed down, carrying on", None),
            completion("measured", None),
            completion("read what came back", None),
        ],
        holding_the_second_call_for_a_third(),
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/lead").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    worker.land_the_rest().unwrap();
    let lines: Vec<serde_json::Value> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .collect();
    let (child, parent) = (
        position_of(&lines, "run_started", "lab/helper"),
        position_of(&lines, "run_frozen", "lab/lead"),
    );
    assert!(
        matches!((child, parent), (Some(child), Some(parent)) if child < parent),
        "the child starts at the call: {child:?} {parent:?}"
    );
}
