// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The waiting send in a running city (collab D9): the sender stops at
//! its safe point, makes no model call, and goes on with the reply.

use kernel::{Address, RunId};

use crate::worker::fixture::*;

fn move_in(city: &std::path::Path, addr: &str) {
    let room = addr
        .split('/')
        .fold(city.to_path_buf(), |at, part| at.join(part));
    std::fs::create_dir_all(&room).unwrap();
    std::fs::write(
        room.join(city::URBANITE_FILE),
        "# URBANITE.md\n\nWorks here.\n",
    )
    .unwrap();
}

/// ito asks hana with `wait`; hana, woken by the signal, replies; ito's
/// wait ends with that reply before ito's run freezes, and the history
/// pairs the start with one reply end.
#[test]
fn a_waiting_sender_goes_on_with_the_reply() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("market").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "market/ito");
    move_in(dir.path(), "market/hana");
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "asking hana",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/hana",
                    "text": "what is your rate?",
                    "wait": true,
                }),
            ),
            tool_completion(
                "answering ito",
                "tu_2",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/ito",
                    "text": "ten coins",
                }),
            ),
            completion("done", None),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    worker.land_the_rest().unwrap();
    let lines: Vec<String> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    let started = at("\"kind\":\"signal_wait_started\"");
    let ended = at("\"kind\":\"signal_wait_ended\"");
    let frozen = lines
        .iter()
        .position(|line| line.contains("\"kind\":\"run_frozen\"") && line.contains("market/ito"));
    assert!(
        matches!((started, ended, frozen), (Some(s), Some(e), Some(f)) if s < e && e < f),
        "the wait starts, ends, then the sender freezes: {lines:#?}"
    );
    if let (Some(s), Some(e)) = (started, ended) {
        let calls = lines[s..e]
            .iter()
            .filter(|line| {
                line.contains("\"kind\":\"model_called\"")
                    && line.contains("\"who\":\"market/ito\"")
            })
            .count();
        assert_eq!(calls, 0, "a waiting run makes no model call");
    }
    let ends: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("\"kind\":\"signal_wait_ended\""))
        .collect();
    assert_eq!(ends.len(), 1, "{ends:?}");
    assert!(ends[0].contains("\"end\":\"reply\""), "{}", ends[0]);
}
