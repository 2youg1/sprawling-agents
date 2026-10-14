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

fn ledger_lines(ledger_dir: &std::path::Path) -> Vec<serde_json::Value> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .collect()
}

/// Whether a run started in `room` itself. The letter a refused child
/// sends names it as the sender, so a run it wakes in the parent's room
/// mentions it too.
fn started_in(lines: &[serde_json::Value], room: &str) -> bool {
    lines
        .iter()
        .any(|line| line["kind"] == "run_started" && line["addr"] == room)
}

fn dispatch_lead(model: Option<&str>) -> wire::Command {
    wire::Command::Dispatch {
        addr: Address::parse("lab/lead").unwrap(),
        task: "get it measured".to_owned(),
        goal: "the number is written down, then stop".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
        session: None,
        effort: None,
        model: model.map(str::to_owned),
    }
}

fn delegating_to(room: &str, id: &str) -> String {
    tool_completion(
        "handing it down",
        id,
        "delegate",
        serde_json::json!({
            "room": room,
            "task": "measure the thing",
            "goal": "a number, then stop",
        }),
    )
}

/// collab D19, accounting D54: the parent runs on a model registered
/// under `digest`, and no model is chosen for `main`, the tag a child
/// that names no model runs on. The room's files admit the child, so
/// the call answers; the hand-over refuses it, and the parent's room
/// receives a stopped handback carrying the refusal's code and
/// recovery instead of nothing.
#[test]
fn a_child_no_attached_model_may_serve_comes_back_to_its_parent_as_a_stopped_handback() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "lab/lead");
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            delegating_to("lab/helper", "tu_1"),
            completion("handed down, carrying on", None),
            completion("read what came back", None),
            completion("read what came back", None),
        ],
    );
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker.read_volume_with(roomy_volume);
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    worker.handle(dispatch_lead(Some("m-local"))).unwrap();
    worker.land_the_rest().unwrap();

    let lines = ledger_lines(&report.ledger_dir);
    assert!(!started_in(&lines, "lab/helper"), "the child never started");
    let handback = lines
        .iter()
        .find(|line| {
            line["kind"] == "signal_enqueued"
                && line.to_string().contains("\"handback\":\"stopped\"")
        })
        .map(ToString::to_string);
    let Some(handback) = handback else {
        panic!("the parent's room received no handback: {lines:?}");
    };
    assert!(handback.contains("\"lab/lead\""), "{handback}");
    assert!(handback.contains("E_MODEL_UNCHOSEN"), "{handback}");
    assert!(handback.contains("pick a model for this tag"), "{handback}");
}

/// collab D18, accounting D53: what the room's files decide is judged
/// before the tool answers. A room that does not parse and a room whose
/// resident names no agent the city knows are refused as the tool
/// result, and no child starts.
#[test]
fn a_delegation_to_a_malformed_room_or_an_unknown_resident_is_refused_as_the_tool_result() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "lab/lead");
    let helper = Address::parse("lab/helper").unwrap();
    let layer = city::config_path(dir.path(), &helper, city::Layer::Resident).unwrap();
    std::fs::create_dir_all(layer.parent().unwrap()).unwrap();
    std::fs::write(&layer, "[resident]\nharness = \"claude\"\n").unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            delegating_to("lab/../elsewhere", "tu_1"),
            delegating_to("lab/helper", "tu_2"),
            completion("told the person", None),
            completion("read what came back", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker.handle(dispatch_lead(None)).unwrap();
    worker.land_the_rest().unwrap();

    let lines = ledger_lines(&report.ledger_dir);
    let answer = |id: &str| {
        lines
            .iter()
            .find(|line| line["kind"] == "tool_result" && line.to_string().contains(id))
            .map(ToString::to_string)
            .unwrap_or_default()
    };
    assert!(
        answer("tu_1").contains("E_INVALID_ARGS"),
        "{}",
        answer("tu_1")
    );
    assert!(
        answer("tu_2").contains("E_CONFIG_INVALID"),
        "{}",
        answer("tu_2")
    );
    assert!(
        !started_in(&lines, "lab/helper"),
        "nothing started in the refused room"
    );
}
