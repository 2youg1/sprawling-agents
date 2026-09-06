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

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

#[test]
fn a_confidential_building_stops_the_run_before_a_remote_call() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    lay_rules(
        dir.path(),
        "vault",
        "# BUILDING.md\n\n## confidential\n\n`confidential: true`\n",
    );

    // A remote endpoint, wired as the provider for a confidential
    // building: the refusal must come from the adapter that could
    // leak, not from a routing table that a mistake could bypass.
    let endpoint = gateway::Endpoint::new(
        gateway::EndpointConfig {
            base_url: "http://127.0.0.1:1/v1/messages".to_owned(),
            dialect: kernel::DialectKind::Anthropic,
            model: "remote".to_owned(),
            auth: gateway::AuthSpec::None,
            extra_headers: Vec::new(),
            overrides: Vec::new(),
            timeout_ms: 1_000,
            pricing: None,
        },
        Box::new(|_reference: &kernel::SecretRef| {
            Err(AxError::failure(
                AxCode::ConfigInvalid,
                "resolve a credential",
                "none configured",
            ))
        }),
    )
    .unwrap();

    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let _ = endpoint;
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&seen);
    worker.observe(Box::new(move |record: &EventRecord| {
        sink.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((record.kind(), serde_json::to_string(record.data()).unwrap()));
    }));
    // The keystroke is accepted; the refusal belongs to the run's own
    // account (drive backstop, card R1.05). What must never happen is
    // a chat POST reaching the endpoint.
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("vault/room1").unwrap(),
            task: "read the private notes".to_owned(),
            goal: "summarise them".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let events = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let denied = events
        .iter()
        .find(|(kind, _)| *kind == EventKind::GateDenied)
        .expect("the refusal is written under its carrier");
    assert!(denied.1.contains("local model"), "{}", denied.1);
    assert!(
        events.iter().any(|(kind, _)| *kind == EventKind::RunFrozen),
        "a refused run still ends"
    );
    assert!(
        !provider
            .exchanges()
            .iter()
            .any(|head| head.starts_with("POST")),
        "no chat call may leave a confidential building"
    );
}
#[test]
fn a_file_an_exec_deleted_comes_back_with_somewhere_to_come_back_from() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let room = dir.path().join("lab").join("room1");
    std::fs::create_dir_all(&room).unwrap();
    std::fs::write(room.join("kiln.md"), "firing notes\n").unwrap();
    let (path, args) = delete_command("kiln.md");
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "clearing up",
                "tu_1",
                "exec",
                serde_json::json!({ "arm": { "program": { "path": path, "args": args } } }),
            ),
            completion("cleared", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "tidy the room".to_owned(),
            goal: "remove the stale note".to_owned(),
            mode: channels::ModeTag::parse("build").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    drop(provider);

    assert!(
        !room.join("kiln.md").exists(),
        "the command ran; if it did not, this test proves nothing"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("file_discarded"),
        "a deletion no forecast saw is still written down: {history}"
    );
    assert!(
        history.contains("restoration"),
        "and it is written down with the way back"
    );
}

/// What the Ledger means by "every effect becomes an EventRecord
/// first", read from the one place where *first* is visible: the
/// write observer, which `memory::jsonl` runs on the appending
/// thread **after** the line is durable.
///
/// One run files a decision on the building's shelf and takes a row
/// from the shared plan. At the instant each line lands, the city
/// must not already carry what that line announces - otherwise a
/// process dying between the two leaves a shelf entry and a claimed
/// row that the history never mentions, and the shelf's own comment
/// ("nothing is on the shelf that the history does not already
/// carry") is false.
fn delete_command(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_owned(),
            vec!["/C".to_owned(), "del".to_owned(), name.to_owned()],
        )
    } else {
        ("rm".to_owned(), vec![name.to_owned()])
    }
}
