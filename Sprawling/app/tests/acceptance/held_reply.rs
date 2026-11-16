// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! F8: a reply one run took and never answered reaches the next run of
//! that room (collab D8, `crates/collab/spec/Delivery.lean`
//! `leave_requeues`).
//!
//! The shape of the run that lost a reply in a test city: a resident
//! pulls the reply a neighbour sent, and the run ends before any model
//! answer that read it is on the ledger. Taking is not consuming, so the
//! reply goes back to the room, the room is knocked, and the resident's
//! next run reads it and is the one recorded as consuming it.
//!
//! Driven through `attend`, the loop a served city runs, with the
//! production workbench and ledger; only the model is scripted. Nothing
//! here reads a platform facility, so Windows, macOS and Linux run the
//! same trace.

use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxCode, AxError, ContentBlock, IdemKey, Model, ModelRequest, ModelReturn, RunId, Seq,
    ToolCall, ToolName,
};

use super::city::{city_with_models, move_in, raise};

/// The reply the neighbour sends, read back by the resident's next run.
const REPLY: &str = "five coins a day";

/// The resident whose run takes the reply and leaves without answering.
const ITO: &str = "market/ito";

/// The neighbour who replies.
const HANA: &str = "market/hana";

#[test]
fn f8_a_reply_taken_by_a_run_that_leaves_reaches_the_rooms_next_run() {
    let dir = tempfile::tempdir().unwrap();
    let (entered_tx, entered) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let heard = Arc::new(Mutex::new(Vec::new()));
    let factory = Turns {
        built: Mutex::new(0),
        entered: Mutex::new(Some(entered_tx)),
        released: Arc::new(Mutex::new(released)),
        heard: Arc::clone(&heard),
    };
    let (mut worker, ledger_dir) = city_with_models(dir.path(), Box::new(factory));
    raise(&mut worker, "market", "minimal");
    move_in(dir.path(), ITO);
    move_in(dir.path(), HANA);
    let desk = Arc::new(accounting::worker::CommandDesk::default());
    let attending = {
        let desk = Arc::clone(&desk);
        std::thread::spawn(move || accounting::worker::attend::attend(&mut worker, &desk))
    };

    desk.post(dispatch(ITO, b"ito"), wire::Reply::nowhere());
    entered
        .recv_timeout(std::time::Duration::from_secs(60))
        .unwrap();
    desk.post(dispatch(HANA, b"hana"), wire::Reply::nowhere());
    wait_for(&ledger_dir, |lines| {
        lines
            .iter()
            .any(|line| line["kind"] == "run_frozen" && mentions(line, HANA))
    });
    release.send(()).unwrap();
    let consumed = wait_for(&ledger_dir, |lines| {
        lines.iter().any(|line| line["kind"] == "signal_consumed")
    });
    desk.close(accounting::worker::Closing::Chosen {
        by: accounting::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });
    attending.join().unwrap();

    let ito_runs: Vec<String> = consumed
        .iter()
        .filter(|line| line["kind"] == "run_started" && mentions(line, ITO))
        .map(|line| line["run"].as_str().unwrap().to_owned())
        .collect();
    let consumers: Vec<String> = consumed
        .iter()
        .filter(|line| line["kind"] == "signal_consumed")
        .map(|line| line["run"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        (ito_runs.len(), consumers.clone()),
        (2, ito_runs.get(1).into_iter().cloned().collect::<Vec<_>>()),
        "the reply is consumed once, by the room's next run: {ito_runs:?} {consumers:?}"
    );
    assert!(
        heard
            .lock()
            .unwrap()
            .iter()
            .any(|said| said.contains(REPLY)),
        "the next run read the reply"
    );
}

fn dispatch(addr: &str, material: &[u8]) -> wire::Command {
    wire::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: "Settle the price.".to_owned(),
        goal: "a price both sides know".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, material),
        session: None,
        effort: None,
        model: None,
    }
}

fn mentions(line: &serde_json::Value, addr: &str) -> bool {
    line.to_string().contains(&format!("\"{addr}\""))
}

/// Reads the history until `done` holds, in counted looks rather than
/// against a clock, which test code may not read.
fn wait_for(
    ledger_dir: &Path,
    done: impl Fn(&[serde_json::Value]) -> bool,
) -> Vec<serde_json::Value> {
    const LOOK: std::time::Duration = std::time::Duration::from_millis(10);
    for _ in 0..6_000 {
        let lines: Vec<serde_json::Value> = runtime::replay::verify_ledger_dir(ledger_dir)
            .map(|chain| {
                chain
                    .raw_lines()
                    .iter()
                    .filter_map(|line| serde_json::from_slice(line).ok())
                    .collect()
            })
            .unwrap_or_default();
        if done(&lines) {
            return lines;
        }
        std::thread::sleep(LOOK);
    }
    panic!("the history never reached the state the test waits for");
}

/// The models of the three runs, in the order the city builds them:
/// the resident's first run, the neighbour's, the resident's next one.
struct Turns {
    built: Mutex<u32>,
    entered: Mutex<Option<mpsc::Sender<()>>>,
    released: Arc<Mutex<mpsc::Receiver<()>>>,
    heard: Arc<Mutex<Vec<String>>>,
}

impl accounting::ModelFactory for Turns {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        let mut built = self.built.lock().unwrap();
        *built = built.saturating_add(1);
        Ok(match *built {
            1 => Box::new(Leaving {
                calls: 0,
                entered: self.entered.lock().unwrap().take(),
                released: Arc::clone(&self.released),
            }),
            2 => Box::new(Replying { calls: 0 }),
            _ => Box::new(Reading {
                calls: 0,
                heard: Arc::clone(&self.heard),
            }),
        })
    }
}

/// Waits in its first call until the reply is in the room, pulls it,
/// and then fails every call, so the run leaves with no answer that
/// read the reply on the ledger.
struct Leaving {
    calls: u32,
    entered: Option<mpsc::Sender<()>>,
    released: Arc<Mutex<mpsc::Receiver<()>>>,
}

impl Model for Leaving {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        if req.chat.tools.is_empty() {
            return says("done");
        }
        self.calls = self.calls.saturating_add(1);
        if self.calls > 1 {
            return Err(AxError::failure(
                AxCode::Provider,
                "call the model",
                "the provider went away mid-run",
            )
            .with_recovery("dispatch again"));
        }
        if let Some(entered) = self.entered.take() {
            entered.send(()).unwrap();
        }
        self.released.lock().unwrap().recv().unwrap();
        calls("signal", serde_json::json!({ "action": "pull" }))
    }
}

/// Sends the reply, then stops.
struct Replying {
    calls: u32,
}

impl Model for Replying {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.calls = self.calls.saturating_add(1);
        if req.chat.tools.is_empty() || self.calls > 1 {
            return says("sent");
        }
        calls(
            "signal",
            serde_json::json!({ "action": "send", "to": ITO, "text": REPLY }),
        )
    }
}

/// Pulls what is waiting, keeps what each tool result said, then stops.
struct Reading {
    calls: u32,
    heard: Arc<Mutex<Vec<String>>>,
}

impl Model for Reading {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.heard.lock().unwrap().extend(
            req.chat
                .messages
                .iter()
                .flat_map(|message| &message.content)
                .filter_map(|block| match block {
                    ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                    ContentBlock::Text { text, .. } => Some(text.clone()),
                    ContentBlock::Thinking { .. }
                    | ContentBlock::RedactedThinking { .. }
                    | ContentBlock::ToolUse { .. }
                    | ContentBlock::Image(_) => None,
                }),
        );
        self.calls = self.calls.saturating_add(1);
        if req.chat.tools.is_empty() || self.calls > 1 {
            return says("read");
        }
        calls("signal", serde_json::json!({ "action": "pull" }))
    }
}

fn calls(tool: &str, args: serde_json::Value) -> Result<ModelReturn, AxError> {
    let id = "call-1".to_owned();
    let name = ToolName::parse(tool)?;
    let args = kernel::Payload::new(args.as_object().cloned().unwrap())?;
    Ok(ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            input: args.clone(),
        }])?,
        vec![ToolCall { id, name, args }],
    ))
}

fn says(text: &str) -> Result<ModelReturn, AxError> {
    Ok(ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::Text {
            text: text.to_owned(),
        }])?,
        Vec::new(),
    ))
}
