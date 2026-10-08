// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A room whose resident is an official harness, dispatched through the
//! person's door. The harness is played in memory on the far side of two
//! pipes, as `agent_protocols`' own session tests play it: no vendor's
//! program starts here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{Address, AxCode, AxError, EventKind, EventRecord};
use serde_json::{Value, json};

use crate::worker::RunWorker;
use crate::worker::driving::harness::{Prompting, StartHarness};

/// What the played agent does once the session has sent it a request.
#[derive(Clone)]
enum Step {
    /// Answer the request just read with this result.
    Answer(Value),
    /// Send this message unprompted.
    Say(Value),
    /// Read one more request from the client.
    Read,
    /// Wait this long for the client's next message: when it comes,
    /// answer the request read last with `heard`, and when it does not,
    /// with `otherwise`.
    Heed {
        within: Duration,
        heard: Value,
        otherwise: Value,
    },
    /// Write a file into the directory the session was opened in.
    Write(&'static str, &'static str),
    /// Read everything the session still sends, until it closes its end.
    Drain,
}

/// What the played agent heard, and where it was opened.
#[derive(Default)]
struct Heard {
    messages: Vec<Value>,
    cwd: Option<PathBuf>,
}

/// A harness that is an agent played over pipes, following `script`,
/// with what it heard kept in `heard`.
fn played(script: Vec<Step>, heard: Arc<Mutex<Heard>>) -> StartHarness {
    Arc::new(move |harness, cwd: &Path| -> Result<Prompting, AxError> {
        let (from_agent, mut agent_out) = std::io::pipe().unwrap();
        let (agent_in, to_agent) = std::io::pipe().unwrap();
        let script = script.clone();
        let heard = Arc::clone(&heard);
        heard.lock().unwrap().cwd = Some(cwd.to_path_buf());
        let root = cwd.to_path_buf();
        let (told, telling) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut lines = BufReader::new(agent_in);
            loop {
                let mut line = String::new();
                if lines.read_line(&mut line).unwrap() == 0 {
                    return;
                }
                let message: Value = serde_json::from_str(&line).unwrap();
                heard.lock().unwrap().messages.push(message.clone());
                if told.send(message).is_err() {
                    return;
                }
            }
        });
        std::thread::spawn(move || {
            let mut last = telling.recv().unwrap();
            for step in script {
                let said = match step {
                    Step::Answer(result) => {
                        json!({ "jsonrpc": "2.0", "id": last["id"], "result": result })
                    }
                    Step::Heed {
                        within,
                        heard,
                        otherwise,
                    } => {
                        let result = match telling.recv_timeout(within) {
                            Ok(_) => heard,
                            Err(_) => otherwise,
                        };
                        json!({ "jsonrpc": "2.0", "id": last["id"], "result": result })
                    }
                    Step::Say(message) => message,
                    Step::Read => {
                        last = telling.recv().unwrap();
                        continue;
                    }
                    Step::Write(file, text) => {
                        let at = root.join(file);
                        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
                        std::fs::write(at, text).unwrap();
                        continue;
                    }
                    Step::Drain => {
                        while telling.recv().is_ok() {}
                        continue;
                    }
                };
                writeln!(agent_out, "{said}").unwrap();
                agent_out.flush().unwrap();
            }
        });
        let name = harness.entry().id.as_str();
        let lines = agent_protocols::Lines::over(BufReader::new(from_agent), name)?;
        let mut session = agent_protocols::AcpSession::open(lines, to_agent, name, cwd)?;
        Ok(Box::new(
            move |text: &str, listener: &mut agent_protocols::Listener<'_>| {
                session.prompt(text, listener)
            },
        ))
    })
}

/// The agent's side of opening a session.
fn opening() -> Vec<Step> {
    vec![
        Step::Answer(json!({ "protocolVersion": 1, "agentCapabilities": {} })),
        Step::Read,
        Step::Answer(json!({ "sessionId": "s1" })),
        Step::Read,
    ]
}

fn said(text: &str) -> Step {
    Step::Say(
        json!({ "jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s1",
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": text } }
        }}),
    )
}

/// A city with the building `lab`, whose layer names `harness`.
fn city_with(harness: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let lab = Address::parse("lab").unwrap();
    let file = city::config_path(dir.path(), &lab, city::Layer::Building).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, format!("[resident]\nharness = \"{harness}\"\n")).unwrap();
    let ledger = report.ledger_dir;
    (dir, file, ledger)
}

fn worker(root: &Path, start: StartHarness) -> RunWorker {
    RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap()
    .with_harnesses(start)
}

fn dispatch(addr: &str, model: Option<&str>) -> wire::Command {
    wire::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: "fix the notes".to_owned(),
        goal: "the notes read well".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
        session: None,
        effort: None,
        model: model.map(str::to_owned),
    }
}

/// A harness that must never start: the dispatch is refused first.
fn never() -> StartHarness {
    Arc::new(|_, _: &Path| -> Result<Prompting, AxError> {
        panic!("a refused dispatch started a harness")
    })
}

/// Every record the city has, in order.
fn records(ledger: &Path) -> Vec<EventRecord> {
    runtime::replay::verify_ledger_dir(ledger)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect()
}

/// A word that names no agent is refused naming every word that does,
/// before the room exists.
#[test]
fn an_unknown_harness_is_refused_naming_the_known_words_and_leaves_no_room_behind() {
    let (dir, file, _) = city_with("claude");
    let mut worker = worker(dir.path(), never());

    assert_eq!(
        worker.handle(dispatch("lab/room1", None)).err(),
        Some(
            AxError::failure(AxCode::ConfigInvalid, "dispatch work", "lab/room1: claude")
                .with_nearby(
                    ["claude_code", "codex", "grok_build", "kimi_code", "pi"]
                        .map(str::to_owned)
                        .to_vec()
                )
                .with_recovery(format!(
                    "write the id of an `[[agent]]` row or of a built-in entry under \
                     `[resident] harness` in {}",
                    file.display()
                ))
        )
    );
    assert!(!dir.path().join("lab").join("room1").exists());
}

/// A confidential building's data does not leave, and a harness sends
/// the room to its vendor.
#[test]
fn a_confidential_building_refuses_a_harness_before_anything_is_written() {
    let (dir, file, _) = city_with("pi");
    crate::worker::fixture::lay_rules(dir.path(), "lab", &crate::worker::fixture::shut_rules(""));
    let mut worker = worker(dir.path(), never());

    assert_eq!(
        worker.handle(dispatch("lab/room1", None)).err(),
        Some(
            AxError::failure(AxCode::GateDenied, "dispatch work", "lab/room1: pi").with_recovery(
                format!(
                    "a harness sends the room to its own vendor and a confidential building's \
                     data does not leave; take `[resident] harness` out of {}, or drop \
                     `confidential = true`",
                    file.display()
                )
            )
        )
    );
    assert!(!dir.path().join("lab").join("room1").exists());
}

/// A dispatch naming a model into a harness room is refused, and the
/// building's layer is left as the person wrote it: no `[model] name`
/// beside the harness.
#[test]
fn a_dispatch_naming_a_model_into_a_harness_room_is_refused_and_writes_no_model() {
    let (dir, file, _) = city_with("pi");
    let before = std::fs::read(&file).unwrap();
    let mut worker = worker(dir.path(), never());

    assert_eq!(
        worker.handle(dispatch("lab/room1", Some("m-local"))).err(),
        Some(
            AxError::failure(
                AxCode::ConfigInvalid,
                "dispatch work",
                "lab/room1: m-local into a room whose resident is pi",
            )
            .with_recovery(format!(
                "dispatch without naming a model, or take `[resident] harness` out of {} to \
                 run m-local here",
                file.display()
            ))
        )
    );
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

/// A harness that will not start is refused before its run exists: the
/// history holds the tree it was lent and no `run_started`.
#[test]
fn a_harness_that_will_not_start_is_refused_before_its_run_exists() {
    let (dir, _, ledger) = city_with("pi");
    let refusal = AxError::failure(AxCode::ToolUnavailable, "start pi", "npx: not found")
        .with_recovery("install the harness and sign in inside it, then dispatch again");
    let refused = refusal.clone();
    let mut worker = worker(
        dir.path(),
        Arc::new(move |_, _: &Path| -> Result<Prompting, AxError> { Err(refused.clone()) }),
    );

    assert_eq!(
        worker.handle(dispatch("lab/room1", None)).err(),
        Some(refusal)
    );
    assert!(
        records(&ledger)
            .iter()
            .all(|record| record.kind() != EventKind::RunStarted),
        "a harness that never started has no run"
    );
}

mod turn;
