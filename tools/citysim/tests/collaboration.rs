// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two collaboration scenarios of TP3 (`tools/citysim/Spec.lean`
//! §16, citysim D25): a coordinator that delegates three rooms, and the
//! turtle-soup game played between `hall/mayor` and `hall/clerk` under
//! the asynchronous and the synchronous send.
//!
//! The city is the product's, served through `attend`, the loop a city
//! runs: lanes, the workbench, the signal desk, the delegate desk and
//! the ledger are production parts. The clock is counted and the vault
//! is in memory; only the model is scripted, and each run's model reads
//! its role from the first message the run was given. Nothing here
//! reads a platform facility, so Windows, macOS and Linux run the same
//! trace up to the order in which concurrent lanes reach the ledger,
//! which no assertion reads.

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

/// The three rooms the coordinator delegates to.
const CHILDREN: [&str; 3] = ["lab/kiln", "lab/glaze", "lab/clay"];

/// How long a child waits for its two siblings to be running too; a
/// lane that ran the children one after another reaches it.
const SIBLINGS: Duration = Duration::from_secs(20);

/// The rounds of questions the second game played.
const ROUNDS: u32 = 3;

/// The host's handbook, read at the start of every run of either seat.
const HANDBOOK: &str = "hall/soup.md";

const MAYOR: &str = "hall/mayor";
const CLERK: &str = "hall/clerk";

#[test]
fn tp3_three_delegated_children_start_at_the_call_and_run_side_by_side() {
    let dir = tempfile::tempdir().unwrap();
    let met = Arc::new(Siblings::default());
    let factory = Roles::new(Arc::clone(&met), Shared::default(), Send::Async);
    let (mut worker, ledger) = city(dir.path(), factory);
    raise(&mut worker, "lab");
    let lines = serve(
        worker,
        &ledger,
        dispatch("lab/lead", "three rooms at work"),
        |lines| frozen(lines) == 4,
    );

    let lead = started(&lines, "lab/lead").remove(0);
    let delegated: Vec<u64> = lines
        .iter()
        .filter(|(record, _)| record.kind() == EventKind::ToolCalled && record.run() == lead.run)
        .map(|(record, _)| record.seq().value())
        .collect();
    let lead_frozen = frozen_seq(&lines, lead.run);
    let children: Vec<Child> = CHILDREN
        .iter()
        .map(|room| {
            let start = started(&lines, room).remove(0);
            Child {
                started: start.seq,
                frozen: frozen_seq(&lines, start.run),
                parent: start.parent == Some(lead.run),
            }
        })
        .collect();
    let report = format!(
        "delegate calls at seq {delegated:?}, lead frozen at {lead_frozen}, children {children:?}, \
         lead started at t={} and children at t={:?}",
        lead.t,
        CHILDREN
            .iter()
            .map(|room| started(&lines, room).remove(0).t)
            .collect::<Vec<_>>()
    );
    println!("{report}");
    assert_eq!(
        (
            children.iter().all(|child| child.parent),
            children
                .iter()
                .zip(&delegated)
                .all(|(child, call)| child.started > *call && child.frozen != u64::MAX),
            met.all_met(),
        ),
        (true, true, true),
        "each child starts at its call, under the lead, and the three run side by side: {report}"
    );
}

/// At most seven runs: the host's first, three guesser runs each started by a
/// question, and three host runs each started by an answer; the self-
/// mention lands in the host's running lane and starts no run. Every
/// run reads the handbook first. Calls: 4 in the first host run (read,
/// send, self-mention, end), 3 in each guesser run (read, answer, end),
/// 3 in each of the two host runs that ask again, 2 in the one that
/// gives the verdict. An answer that reaches the host while its
/// previous run is still at work lands at that run's next safe point
/// and starts no run (TP3 B), so one run fewer is the other order the
/// lanes may take.
#[test]
fn tp3_the_second_game_under_the_asynchronous_send() {
    let reading = play(Send::Async);
    println!("asynchronous: {reading:?}");
    let serial = 1 + 2 * ROUNDS;
    assert!(
        reading.runs == reading.reads_at_start
            && (serial - 1..=serial).contains(&reading.runs)
            && reading.model_calls <= 7 * 2 + 2 * ROUNDS + 1,
        "{reading:?}"
    );
}

/// Four runs: one host run that waits at each question, and three
/// guesser runs. The host reads once, asks three times, and gives the
/// verdict; a wait makes no model call.
#[test]
fn tp3_the_second_game_under_the_synchronous_send() {
    let reading = play(Send::Sync);
    println!("synchronous: {reading:?}");
    assert_eq!(
        reading.counts(),
        Counts {
            runs: 1 + ROUNDS,
            reads_at_start: 1 + ROUNDS,
            model_calls: 1 + ROUNDS + 1 + ROUNDS * 3,
        },
        "{reading:?}"
    );
}

#[derive(Debug)]
struct Child {
    started: u64,
    frozen: u64,
    parent: bool,
}

/// What one game left behind: the counts a TP3 reading compares, and
/// the counted-clock span from the first run's start to the last
/// freeze, which counts clock reads rather than milliseconds.
#[derive(Debug)]
struct Reading {
    runs: u32,
    reads_at_start: u32,
    model_calls: u32,
    span_reads: u64,
    seq_span: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Counts {
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

fn play(send: Send) -> Reading {
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
        dispatch(MAYOR, "the game is judged"),
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
enum Send {
    Async,
    Sync,
}

/// The game's state both seats read: the rounds asked and answered,
/// whether the verdict was given, and what the models were asked.
#[derive(Clone, Default)]
struct Shared {
    asked: Arc<AtomicU32>,
    answered: Arc<AtomicU32>,
    verdict: Arc<AtomicU32>,
    calls: Arc<AtomicU32>,
    reads_at_start: Arc<AtomicU32>,
    detoured: Arc<AtomicU32>,
}

/// Three children that each wait until all three are running.
#[derive(Default)]
struct Siblings {
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
struct Roles {
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
enum Role {
    Coordinator,
    Child,
    Host,
    Guesser,
}

struct Seat {
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
            return calls("read", json!({ "path": HANDBOOK }));
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
                calls("read", json!({ "path": HANDBOOK }))
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
fn role_of(req: &ModelRequest) -> Role {
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

fn calls(tool: &str, args: Value) -> Result<ModelReturn, AxError> {
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

fn says(text: &str) -> Result<ModelReturn, AxError> {
    Ok(ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::Text {
            text: text.to_owned(),
        }])?,
        Vec::new(),
    ))
}

/// A clock that counts the times it is read, from a fixed start.
struct Counted(AtomicU64);

impl accounting::Clock for Counted {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(
            1_790_000_000_000_u64.saturating_add(self.0.fetch_add(1, Ordering::Relaxed)),
        ))
    }
}

fn hands(clock: &Arc<dyn accounting::Clock + std::marker::Send + Sync>) -> Hands {
    Hands {
        clock: Arc::clone(clock),
        ..sprawling::assembly::hands(gateway::Custodian::in_memory())
    }
}

fn idem(what: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, what)
}

/// A founded city whose main model sits behind a loopback port nothing
/// listens on, so only the roles answer; with its ledger directory.
fn city(dir: &Path, factory: Roles) -> (RunWorker, PathBuf) {
    let clock: Arc<dyn accounting::Clock + std::marker::Send + Sync> =
        Arc::new(Counted(AtomicU64::new(0)));
    accounting::worker::genesis::form(
        dir,
        accounting::worker::genesis::Adopt::Nothing,
        hands(&clock),
    )
    .unwrap();
    let mut worker = RunWorker::new(dir, runtime::diagnostics::Diagnostics::off(), hands(&clock))
        .unwrap()
        .with_models(Box::new(factory));
    let refusing = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}/v1", refusing.local_addr().unwrap());
    drop(refusing);
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["scripted".to_owned()],
            tuning: wire::EndpointTuning {
                timeout_ms: Some(2_000),
                request_max_retries: Some(0),
                ..wire::EndpointTuning::default()
            },
            idem: idem(b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint,
            model: "scripted".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(131_072),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: idem(b"select"),
        })
        .unwrap();
    (worker, CityLayout::new(dir).ledger())
}

fn raise(worker: &mut RunWorker, addr: &str) {
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse(addr).unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: idem(addr.as_bytes()),
        })
        .unwrap();
}

fn path_of(dir: &Path, addr: &str) -> PathBuf {
    addr.split('/')
        .fold(dir.to_path_buf(), |at, part| at.join(part))
}

fn move_in(dir: &Path, addr: &str) {
    let urbanite = CityLayout::new(dir).urbanite(&Address::parse(addr).unwrap());
    std::fs::create_dir_all(urbanite.parent().unwrap()).unwrap();
    std::fs::write(urbanite, "# URBANITE.md\n\nWorks here.\n").unwrap();
}

fn dispatch(addr: &str, goal: &str) -> wire::Command {
    wire::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: "Host the turtle soup, or delegate three rooms.".to_owned(),
        goal: goal.to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, goal.as_bytes()),
        session: None,
        effort: None,
        model: None,
    }
}

type Lines = Vec<(EventRecord, String)>;

/// Serves the city through `attend`, posts `command`, and reads the
/// history until `done` holds, in counted looks.
fn serve(
    mut worker: RunWorker,
    ledger: &Path,
    command: wire::Command,
    done: impl Fn(&Lines) -> bool,
) -> Lines {
    let desk = Arc::new(CommandDesk::default());
    let attending = {
        let desk = Arc::clone(&desk);
        std::thread::spawn(move || accounting::worker::attend::attend(&mut worker, &desk))
    };
    desk.post(command, wire::Reply::nowhere());
    for _ in 0..3_000 {
        if done(&read(ledger)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    desk.close(Closing::Chosen);
    attending.join().unwrap();
    let lines = read(ledger);
    let trace: Vec<String> = lines
        .iter()
        .filter(|(record, _)| {
            matches!(
                record.kind(),
                EventKind::RunStarted
                    | EventKind::RunFrozen
                    | EventKind::ToolCalled
                    | EventKind::SignalEnqueued
                    | EventKind::ToolResult
            )
        })
        .map(|(record, raw)| {
            format!(
                "{}:{:?}@{} {}",
                record.seq().value(),
                record.kind(),
                record.addr().map_or("-", Address::as_str),
                raw.get(raw.len().saturating_sub(120)..).unwrap_or_default()
            )
        })
        .collect();
    assert!(
        done(&lines),
        "the history never reached the end the scenario waits for: {trace:?}"
    );
    lines
}

fn read(ledger: &Path) -> Lines {
    runtime::replay::verify_ledger_dir(ledger)
        .map(|chain| {
            chain
                .raw_lines()
                .iter()
                .filter_map(|line| {
                    EventRecord::parse_line(line)
                        .ok()
                        .map(|record| (record, String::from_utf8_lossy(line).into_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

struct Start {
    run: RunId,
    seq: u64,
    t: u64,
    parent: Option<RunId>,
}

fn started(lines: &Lines, room: &str) -> Vec<Start> {
    lines
        .iter()
        .filter(|(record, _)| {
            record.kind() == EventKind::RunStarted
                && record.addr().map(Address::as_str) == Some(room)
        })
        .map(|(record, _)| {
            let data: RunStarted = record.data().read().unwrap();
            Start {
                run: record.run(),
                seq: record.seq().value(),
                t: record.t().value(),
                parent: data.parent,
            }
        })
        .collect()
}

fn frozen(lines: &Lines) -> usize {
    lines
        .iter()
        .filter(|(record, _)| record.kind() == EventKind::RunFrozen)
        .count()
}

/// Every run that started has frozen.
fn quiet(lines: &Lines) -> bool {
    let starts = lines
        .iter()
        .filter(|(record, _)| record.kind() == EventKind::RunStarted)
        .count();
    starts > 0 && starts == frozen(lines)
}

fn frozen_seq(lines: &Lines, run: RunId) -> u64 {
    lines
        .iter()
        .find(|(record, _)| record.kind() == EventKind::RunFrozen && record.run() == run)
        .map_or(u64::MAX, |(record, _)| record.seq().value())
}
