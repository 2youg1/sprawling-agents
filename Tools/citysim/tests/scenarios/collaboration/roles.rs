// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code,
    reason = "test code; readings are read through their Debug form in the report"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use accounting::worker::hands::Hands;
use accounting::worker::{Closing, CommandDesk, RunWorker};
use kernel::event::record::RunStarted;
use kernel::layout::CityLayout;
use kernel::{
    Address, AxError, ContentBlock, EventKind, EventRecord, IdemKey, Model, ModelRequest,
    ModelReturn, Payload, RunId, Seq, TimeMs, ToolCall, ToolName,
};
use serde_json::{Value, json};

pub(super) const CHILDREN: [&str; 3] = ["lab/kiln", "lab/glaze", "lab/clay"];

/// How long a child waits for its two siblings to be running too; a
/// lane that ran the children one after another reaches it.
pub(super) const SIBLINGS: Duration = Duration::from_secs(20);

/// The rounds of questions the second game played.
pub(super) const ROUNDS: u32 = 3;

/// The host's handbook, read at the start of every run of either seat.
pub(super) const HANDBOOK: &str = "hall/soup.md";

pub(super) const MAYOR: &str = "hall/mayor";
pub(super) const CLERK: &str = "hall/clerk";

pub(super) struct Child {
    started: u64,
    frozen: u64,
    parent: bool,
}

/// What one game left behind: the counts a TP3 reading compares, and
/// the counted-clock span from the first run's start to the last
/// freeze, which counts clock reads rather than milliseconds.
#[derive(Debug)]
pub(super) struct Reading {
    runs: u32,
    reads_at_start: u32,
    model_calls: u32,
    span_reads: u64,
    seq_span: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Counts {
    runs: u32,
    reads_at_start: u32,
    model_calls: u32,
}

impl Reading {
    fn counts(&self) -> Counts {
        Counts {
            runs: self.runs,
            reads_at_start: self.reads_at_start,
            model_calls: self.model_calls,
        }
    }
}

pub(super) fn play(send: Send) -> Reading {
    let dir = tempfile::tempdir().unwrap();
    let shared = Shared::default();
    let factory = Roles::new(Arc::default(), shared.clone(), send);
    let (worker, ledger) = city(dir.path(), factory);
    move_in(dir.path(), MAYOR);
    move_in(dir.path(), CLERK);
    std::fs::write(
        path_of(dir.path(), HANDBOOK),
        "# Turtle soup\n\nYes, no or irrelevant; three rounds.\n",
    )
    .unwrap();
    let done = shared.clone();
    let lines = serve(
        worker,
        &ledger,
        vec![dispatch(MAYOR, "the game is judged")],
        move |lines| done.verdict.load(Ordering::SeqCst) > 0 && quiet(lines),
    );
    let starts: Vec<&EventRecord> = lines
        .iter()
        .map(|(record, _)| record)
        .filter(|record| record.kind() == EventKind::RunStarted)
        .collect();
    let first = starts.first().map_or(0, |record| record.t().value());
    let (last_t, last_seq) = lines
        .iter()
        .rev()
        .map(|(record, _)| record)
        .find(|record| record.kind() == EventKind::RunFrozen)
        .map_or((0, 0), |record| (record.t().value(), record.seq().value()));
    Reading {
        runs: u32::try_from(starts.len()).unwrap(),
        reads_at_start: shared.reads_at_start.load(Ordering::SeqCst),
        model_calls: shared.calls.load(Ordering::SeqCst),
        span_reads: last_t.saturating_sub(first),
        seq_span: last_seq.saturating_sub(starts.first().map_or(0, |record| record.seq().value())),
    }
}

/// Whether the send waits for the reply (C's synchronous arm) or goes
/// on at once (the default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Send {
    Async,
    Sync,
}

/// The game's state both seats read: the rounds asked and answered,
/// whether the verdict was given, and what the models were asked.
#[derive(Clone, Default)]
pub(super) struct Shared {
    asked: Arc<AtomicU32>,
    answered: Arc<AtomicU32>,
    verdict: Arc<AtomicU32>,
    calls: Arc<AtomicU32>,
    reads_at_start: Arc<AtomicU32>,
    detoured: Arc<AtomicU32>,
}

/// Three children that each wait until all three are running.
#[derive(Default)]
pub(super) struct Siblings {
    running: Mutex<u32>,
    arrived: Condvar,
    met: AtomicU32,
}

impl Siblings {
    fn arrive(&self) {
        let mut running = self.running.lock().unwrap();
        *running = running.saturating_add(1);
        self.arrived.notify_all();
        let (running, waited) = self
            .arrived
            .wait_timeout_while(running, SIBLINGS, |running| *running < 3)
            .unwrap();
        drop(running);
        if !waited.timed_out() {
            self.met.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn all_met(&self) -> bool {
        self.met.load(Ordering::SeqCst) == 3
    }
}

/// Builds every run's model; the model reads its role on its first call.
pub(super) struct Roles {
    siblings: Arc<Siblings>,
    shared: Shared,
    send: Send,
}

impl Roles {
    fn new(siblings: Arc<Siblings>, shared: Shared, send: Send) -> Roles {
        Roles {
            siblings,
            shared,
            send,
        }
    }
}

impl accounting::ModelFactory for Roles {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + std::marker::Send>, AxError> {
        Ok(Box::new(Seat {
            role: None,
            turn: 0,
            siblings: Arc::clone(&self.siblings),
            shared: self.shared.clone(),
            send: self.send,
        }))
    }
}

/// Which part a run plays, read from the first message it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Coordinator,
    Child,
    Host,
    Guesser,
}

pub(super) struct Seat {
    role: Option<Role>,
    turn: u32,
    siblings: Arc<Siblings>,
    shared: Shared,
    send: Send,
}

impl Model for Seat {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        if req.chat.tools.is_empty() {
            return says("done");
        }
        self.shared.calls.fetch_add(1, Ordering::SeqCst);
        let role = *self.role.get_or_insert_with(|| role_of(req));
        self.turn = self.turn.saturating_add(1);
        match role {
            Role::Coordinator => self.coordinate(),
            Role::Child => {
                self.siblings.arrive();
                says("measured")
            }
            Role::Host => self.host(),
            Role::Guesser => self.guess(),
        }
    }
}

impl Seat {
    fn coordinate(&self) -> Result<ModelReturn, AxError> {
        let Some(room) = usize::try_from(self.turn)
            .ok()
            .and_then(|turn| turn.checked_sub(1))
            .and_then(|at| CHILDREN.get(at))
        else {
            return says("three rooms are at work");
        };
        calls(
            "delegate",
            json!({ "room": room, "task": "measure one thing", "goal": "a number, then stop" }),
        )
    }

    /// Reads the handbook, asks the next round, and gives the verdict
    /// once every round is answered. Under the asynchronous send the
    /// first run also mentions itself, the detour the test city took to
    /// look again later; a run that finds its question still open ends.
    fn host(&self) -> Result<ModelReturn, AxError> {
        if self.turn == 1 {
            self.shared.reads_at_start.fetch_add(1, Ordering::SeqCst);
            return calls("read", json!({ "file_path": HANDBOOK }));
        }
        let asked = self.shared.asked.load(Ordering::SeqCst);
        let answered = self.shared.answered.load(Ordering::SeqCst);
        if self.send == Send::Async
            && self.turn == 3
            && self.shared.detoured.fetch_add(1, Ordering::SeqCst) == 0
        {
            return calls(
                "signal",
                json!({ "action": "send", "to": MAYOR, "text": "look again later" }),
            );
        }
        if answered >= ROUNDS {
            if self.shared.verdict.fetch_add(1, Ordering::SeqCst) == 0 {
                return says("verdict: solved");
            }
            return says("the game is over");
        }
        if asked > answered || (self.send == Send::Async && self.turn > 2) {
            return says("waiting for the guesser");
        }
        self.shared.asked.fetch_add(1, Ordering::SeqCst);
        let mut args = json!({
            "action": "send",
            "to": CLERK,
            "text": format!("round {}: ask up to four questions", asked.saturating_add(1)),
        });
        if self.send == Send::Sync {
            args["wait"] = json!(true);
        }
        calls("signal", args)
    }

    /// Reads the handbook, answers the round with its questions, ends.
    fn guess(&self) -> Result<ModelReturn, AxError> {
        match self.turn {
            1 => {
                self.shared.reads_at_start.fetch_add(1, Ordering::SeqCst);
                calls("read", json!({ "file_path": HANDBOOK }))
            }
            2 => {
                self.shared.answered.fetch_add(1, Ordering::SeqCst);
                calls(
                    "signal",
                    json!({ "action": "send", "to": MAYOR, "text": "four questions" }),
                )
            }
            _ => says("asked"),
        }
    }
}

/// The part a run plays: a signal from the host makes a guesser and one
/// from anyone else a host; a delegated task makes a child.
pub(super) fn role_of(req: &ModelRequest) -> Role {
    let first: String = req
        .chat
        .messages
        .first()
        .into_iter()
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            ContentBlock::Text { text, .. } => Some(text.as_str()),
            ContentBlock::ToolResult { .. }
            | ContentBlock::Thinking { .. }
            | ContentBlock::RedactedThinking { .. }
            | ContentBlock::ToolUse { .. }
            | ContentBlock::Image(_) => None,
        })
        .chain(req.chat.system.iter().map(|block| block.text.as_str()))
        .collect();
    if first.contains(&format!("@{MAYOR} signalled you")) && !first.contains("look again") {
        Role::Guesser
    } else if first.contains("a number, then stop") {
        Role::Child
    } else if first.contains("three rooms at work") {
        Role::Coordinator
    } else {
        Role::Host
    }
}

pub(super) fn calls(tool: &str, args: Value) -> Result<ModelReturn, AxError> {
    let id = "call-1".to_owned();
    let name = ToolName::parse(tool)?;
    let args = Payload::new(args.as_object().cloned().unwrap())?;
    Ok(ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            input: args.clone(),
        }])?,
        vec![ToolCall { id, name, args }],
    ))
}

pub(super) fn says(text: &str) -> Result<ModelReturn, AxError> {
    Ok(ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::Text {
            text: text.to_owned(),
        }])?,
        Vec::new(),
    ))
}

/// A clock that counts the times it is read, from a fixed start.
pub(super) struct Counted(AtomicU64);

impl accounting::Clock for Counted {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(
            1_790_000_000_000_u64.saturating_add(self.0.fetch_add(1, Ordering::Relaxed)),
        ))
    }
}

pub(super) fn hands(clock: &Arc<dyn accounting::Clock + std::marker::Send + Sync>) -> Hands {
    Hands {
        clock: Arc::clone(clock),
        ..sprawling::assembly::hands(gateway::Custodian::in_memory())
    }
}

pub(super) fn idem(what: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, what)
}
