// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The checks derived from `crates/runtime/spec/Turn/Durability.lean`
//! (runtime D24): how many barriers a turn pays, that a write's intent
//! is durable before the write runs, and what a power cut at any line of
//! the ledger's write sequence leaves on disk.

use std::sync::{Arc, Mutex};

use super::super::*;
use super::helpers::*;
use crate::bench::{BenchOutcome, Clearance, Ticket, ToolBench};
use crate::conversation::Opening;
use kernel::{Effect, EventDraft, EventRef, IdemKey, Seq, Tool, ToolOutcome};
use proptest::prelude::*;

/// The kinds of the lines a ledger holds, shared with the tools so a
/// tool can read what is durable at the moment it runs.
type Durable = Arc<Mutex<Vec<String>>>;

/// A ledger that counts its barriers: each call into it is one round
/// trip through the relay, and so one disk barrier. The power goes out
/// when it is about to write line `cut_at` (counted from zero): that line
/// and every later one never reach the disk.
struct Barriers {
    inner: TestLedger,
    durable: Durable,
    barriers: usize,
    cut_at: usize,
}

impl Barriers {
    fn new() -> Barriers {
        Barriers::cut_at(usize::MAX)
    }

    fn cut_at(line: usize) -> Barriers {
        Barriers {
            inner: TestLedger::new(),
            durable: Arc::default(),
            barriers: 0,
            cut_at: line,
        }
    }

    fn keep(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        if self.durable.lock().unwrap().len() >= self.cut_at {
            return Err(AxError::failure(
                AxCode::Busy,
                "append to the ledger",
                "the power went out",
            )
            .with_recovery("open the ledger again; it holds what reached the disk"));
        }
        let kind = serde_json::to_value(draft.kind)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let echo = self.inner.append(draft)?;
        self.durable.lock().unwrap().push(kind);
        Ok(echo)
    }
}

impl Ledger for Barriers {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        self.barriers = self.barriers.saturating_add(1);
        self.keep(draft)
    }

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        self.barriers = self.barriers.saturating_add(1);
        drafts.into_iter().map(|draft| self.keep(draft)).collect()
    }
}

/// A tool that answers with nothing and, when it watches, writes down
/// what was durable each time it ran.
struct Witness {
    meta: kernel::ToolMeta,
    durable: Durable,
    saw: Arc<Mutex<Vec<Vec<String>>>>,
}

impl Tool for Witness {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.meta
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let now = self.durable.lock().unwrap().clone();
        self.saw.lock().unwrap().push(now);
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

fn meta(name: &str, effect: Effect) -> kernel::ToolMeta {
    kernel::ToolMeta {
        name: kernel::ToolName::parse(name).unwrap(),
        disclosure: format!("the {name} tool"),
        params: Payload::empty(),
        effect,
        cost_tier: kernel::CostTier::Free,
        timeout: None,
        render: kernel::RenderIntent::Generic,
        temporal: kernel::Temporal::Timeless,
    }
}

/// A face over a bench holding `read` and `write`. The bench clears both
/// as reads, so no door stands in the way; the face names `write` as a
/// write, which is what decides the wave's order.
struct Face {
    bench: ToolBench,
    write: kernel::ToolMeta,
    next: u64,
}

impl Face {
    fn new(durable: &Durable, saw: &Arc<Mutex<Vec<Vec<String>>>>) -> Face {
        let domain =
            kernel::WriteDomain::new(vec![kernel::Address::parse("lab").unwrap()]).unwrap();
        let mut bench = ToolBench::new(domain);
        for name in ["read", "write"] {
            bench
                .register(Box::new(Witness {
                    meta: meta(name, Effect::Read),
                    durable: Arc::clone(durable),
                    saw: Arc::clone(saw),
                }))
                .unwrap();
        }
        Face {
            bench,
            write: meta(
                "write",
                Effect::Write {
                    domain: kernel::Address::parse("lab").unwrap(),
                },
            ),
            next: 0,
        }
    }
}

fn answer(outcome: BenchOutcome) -> Result<ToolOutcome, AxError> {
    match outcome {
        BenchOutcome::Ran { outcome, .. } | BenchOutcome::Duplicate { outcome } => Ok(outcome),
        BenchOutcome::Refused { refusal } => Err(*refusal),
    }
}

impl ConcurrentInvoke for Face {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        if call.name.as_str() == "write" {
            return Some(&self.write);
        }
        self.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        let at = self.next;
        self.next = at.saturating_add(1);
        let key = IdemKey::derive(&run_id(), Seq::new(at), b"durability");
        match self.bench.clear(call, &key, t) {
            Ok(Clearance::Cleared(ticket)) => Admitted::Cleared(ticket),
            Ok(Clearance::Answered(outcome)) => Admitted::Answered(answer(outcome)),
            Err(refused) => Admitted::Answered(Err(refused)),
        }
    }

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.bench.account(ticket, answered).and_then(answer)
    }
}

fn call(id: &str, tool: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: kernel::ToolName::parse(tool).unwrap(),
        args: Payload::empty(),
    }
}

/// One whole turn whose model asks for `calls`, on `ledger`; what each
/// tool saw durable when it ran comes back in call order.
fn turn_of(ledger: &mut Barriers, calls: Vec<ToolCall>) -> (TurnReport, Vec<Vec<String>>) {
    let saw = Arc::default();
    let report = run_turn(ledger, calls, &saw).unwrap();
    let saw = saw.lock().unwrap().clone();
    (report, saw)
}

/// The same turn, ending on the ledger's refusal instead of unwrapping it.
fn run_turn(
    ledger: &mut Barriers,
    calls: Vec<ToolCall>,
    saw: &Arc<Mutex<Vec<Vec<String>>>>,
) -> Result<TurnReport, AxError> {
    let mut face = Face::new(&ledger.durable, saw);
    let mut model = OneShotModel { calls };
    let mut conversation = Conversation::new();
    conversation.push_task_lines("read and write", "a mixed wave", Opening::FromJob);
    let turn = advance(opened::<1>().assemble(
        Interrupt::None,
        ledger,
        RunPrompt::new(&prefix(), &mut PromptRecord::default()),
        &conversation,
        &[],
        &shape(),
    )?);
    let turn = advance(turn.call(
        Interrupt::None,
        ledger,
        &mut model,
        &BuildingPolicy::default(),
        Generating::Unwatched,
    )?);
    let turn = advance(
        turn.execute_concurrent(Interrupt::None, ledger, &mut face, &mut |_| Interrupt::None)?,
    );
    Ok(advance(turn.record(Interrupt::None, ledger)?))
}

/// `tf1_write_intent_durable`: when a write runs, its own `tool_called`
/// and every line before it are already on the ledger, while the read
/// before it ran without waiting for its own or for `model_returned`.
#[test]
fn tf1_write_intent_is_durable_before_the_write_runs() {
    let mut ledger = Barriers::new();
    let (_, saw) = turn_of(
        &mut ledger,
        vec![call("r1", "read"), call("w1", "write"), call("r2", "read")],
    );
    let opening = ["prompt_assembled", "model_called"];
    let at_write = [
        "prompt_assembled",
        "model_called",
        "model_returned",
        "tool_called",
        "tool_result",
        "tool_called",
    ];
    assert_eq!(
        saw,
        vec![opening.to_vec(), at_write.to_vec(), at_write.to_vec()]
    );
}

/// `closed_turn_barriers`: a turn pays one barrier for its model call,
/// which carries a first turn's `prompt_assembled` too, one for each
/// write, and one to close, however many reads it makes; the report's
/// refs are every line the turn wrote, all durable.
#[test]
fn tf1_turn_barriers() {
    let mut ledger = Barriers::new();
    let (report, _) = turn_of(
        &mut ledger,
        vec![
            call("r1", "read"),
            call("r2", "read"),
            call("w1", "write"),
            call("r3", "read"),
            call("w2", "write"),
        ],
    );
    assert_eq!(
        (
            ledger.barriers,
            report.refs().len(),
            ledger.inner.lines.len()
        ),
        (2 + 2, 13, 13)
    );
}

/// The wave a model asks for: `true` is a write, `false` a read.
fn wave_of(writes: &[bool]) -> Vec<ToolCall> {
    writes
        .iter()
        .enumerate()
        .map(|(i, write)| match write {
            true => call(&format!("w{i}"), "write"),
            false => call(&format!("r{i}"), "read"),
        })
        .collect()
}

proptest! {
    /// `tf1_write_intent_durable` and `tf1_durable_is_reference_prefix`
    /// under a power cut at any line of the ledger's write sequence:
    /// every write that ran saw its own `tool_called` as the last durable
    /// line, with one `tool_called` per call before it, and what reached
    /// the disk is a prefix of the lines the uncut turn writes.
    #[test]
    fn tf1_a_power_cut_leaves_a_prefix_and_no_write_without_its_intent(
        writes in proptest::collection::vec(any::<bool>(), 0..8),
        cut in 0usize..24,
    ) {
        let mut whole = Barriers::new();
        run_turn(&mut whole, wave_of(&writes), &Arc::default()).unwrap();
        let serial = whole.durable.lock().unwrap().clone();

        let mut ledger = Barriers::cut_at(cut);
        let saw = Arc::default();
        let ended = run_turn(&mut ledger, wave_of(&writes), &saw);
        let durable = ledger.durable.lock().unwrap().clone();
        prop_assert!(serial.starts_with(&durable));
        prop_assert_eq!(ended.is_ok(), durable.len() == serial.len());

        let ran = saw.lock().unwrap().clone();
        for (i, seen) in ran.iter().enumerate() {
            if writes[i] {
                let intents = seen.iter().filter(|kind| *kind == "tool_called").count();
                prop_assert_eq!(seen.last().map(String::as_str), Some("tool_called"));
                prop_assert_eq!(Some(intents), i.checked_add(1));
            }
        }
    }
}
