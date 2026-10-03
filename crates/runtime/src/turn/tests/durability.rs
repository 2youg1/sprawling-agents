// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The checks derived from `crates/runtime/spec/Turn/Durability.lean`
//! (runtime D24): how many barriers a turn pays, and that a write's
//! intent is durable before the write runs.

use std::sync::{Arc, Mutex};

use super::super::*;
use super::helpers::*;
use crate::bench::{BenchOutcome, Clearance, Ticket, ToolBench};
use crate::conversation::Opening;
use kernel::{Effect, EventDraft, EventRef, IdemKey, Seq, Tool, ToolOutcome};

/// The kinds of the lines a ledger holds, shared with the tools so a
/// tool can read what is durable at the moment it runs.
type Durable = Arc<Mutex<Vec<String>>>;

/// A ledger that counts its barriers: each call into it is one round
/// trip through the relay, and so one disk barrier.
struct Barriers {
    inner: TestLedger,
    durable: Durable,
    barriers: usize,
}

impl Barriers {
    fn new() -> Barriers {
        Barriers {
            inner: TestLedger::new(),
            durable: Arc::default(),
            barriers: 0,
        }
    }

    fn keep(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
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
    let mut face = Face::new(&ledger.durable, &saw);
    let mut model = OneShotModel { calls };
    let mut conversation = Conversation::new();
    conversation.push_task_lines("read and write", "a mixed wave", Opening::FromJob);
    let turn = advance(
        opened::<1>()
            .assemble(
                Interrupt::None,
                ledger,
                RunPrompt::new(&prefix(), &mut PromptRecord::default()),
                &conversation,
                &[],
                &shape(),
            )
            .unwrap(),
    );
    let turn = advance(
        turn.call(
            Interrupt::None,
            ledger,
            &mut model,
            &BuildingPolicy::default(),
            Generating::Unwatched,
        )
        .unwrap(),
    );
    let turn = advance(
        turn.execute_concurrent(Interrupt::None, ledger, &mut face, &mut |_| Interrupt::None)
            .unwrap(),
    );
    let report = advance(turn.record(Interrupt::None, ledger).unwrap());
    let saw = saw.lock().unwrap().clone();
    (report, saw)
}

/// `tf1_write_intent_durable`: when a write runs, its own `tool_called`
/// and every line before it are already on the ledger, while the read
/// before it ran without waiting for its own.
#[test]
fn tf1_write_intent_is_durable_before_the_write_runs() {
    let mut ledger = Barriers::new();
    let (_, saw) = turn_of(
        &mut ledger,
        vec![call("r1", "read"), call("w1", "write"), call("r2", "read")],
    );
    let opening = ["prompt_assembled", "model_called", "model_returned"];
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
/// one for the reply, one for each write, and one to close, however many
/// reads it makes, plus one on a run's first turn for `prompt_assembled`;
/// the report's refs are every line the turn wrote, all durable.
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
        (1 + 3 + 2, 13, 13)
    );
}
