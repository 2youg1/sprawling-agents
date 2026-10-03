// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::disallowed_methods)]

use super::super::*;
use super::helpers::*;
use crate::bench::{BenchOutcome, Clearance, Ticket, ToolBench};
use crate::conversation::Opening;
use kernel::{Effect, IdemKey, Seq, Tool, ToolOutcome};

/// A read-only tool that answers every call with an empty payload.
struct Read(kernel::ToolMeta);

impl Tool for Read {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.0
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

/// A bench holding `read`, its calls keyed by their position in the
/// wave, through the three stages a concurrent wave uses.
pub(super) struct Placed {
    pub(super) bench: ToolBench,
    next: u64,
}

impl Placed {
    pub(super) fn new() -> Placed {
        let domain =
            kernel::WriteDomain::new(vec![kernel::Address::parse("lab").unwrap()]).unwrap();
        let mut bench = ToolBench::new(domain);
        bench
            .register(Box::new(Read(kernel::ToolMeta {
                name: kernel::ToolName::parse("read").unwrap(),
                disclosure: "reads a file".to_owned(),
                params: Payload::empty(),
                effect: Effect::Read,
                cost_tier: kernel::CostTier::Free,
                timeout: None,
                render: kernel::RenderIntent::Generic,
                temporal: kernel::Temporal::Timeless,
            })))
            .unwrap();
        Placed { bench, next: 0 }
    }

    fn key(&mut self) -> IdemKey {
        let at = self.next;
        self.next = at.saturating_add(1);
        IdemKey::derive(&run_id(), Seq::new(at), b"read")
    }

    /// The same calls one after another, each answered as it is
    /// admitted: serial, with the registration still known.
    fn one_by_one(&mut self, call: &ToolCall, t: TimeMs) -> Result<ToolOutcome, AxError> {
        let key = self.key();
        self.bench.invoke(call, &key, t).and_then(answer)
    }
}

/// The serial reference: a face that runs each call where it admits it,
/// so no read starts early, and that still names each tool's
/// registration, as the lane's own face does.
struct OneByOne(Placed);

impl ConcurrentInvoke for OneByOne {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.0.meta_of(call)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        Admitted::Answered(self.0.one_by_one(call, t))
    }

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.0.tool(ticket)
    }

    fn account(
        &mut self,
        call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.0.account(call, ticket, answered)
    }
}

fn answer(outcome: BenchOutcome) -> Result<ToolOutcome, AxError> {
    match outcome {
        BenchOutcome::Ran { outcome, .. } | BenchOutcome::Duplicate { outcome } => Ok(outcome),
        BenchOutcome::Refused { refusal } => Err(*refusal),
    }
}

impl ConcurrentInvoke for Placed {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        let key = self.key();
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

pub(super) fn wave_of(ledger: &mut TestLedger, calls: Vec<ToolCall>) -> Turn<'static, ToolWave> {
    waved(opened::<1>(), ledger, calls)
}

/// `turn`, assembled and called until it holds a wave of `calls`.
pub(super) fn waved<'h>(
    turn: Turn<'h, Assembling>,
    ledger: &mut TestLedger,
    calls: Vec<ToolCall>,
) -> Turn<'h, ToolWave> {
    let mut model = OneShotModel { calls };
    let mut conversation = Conversation::new();
    conversation.push_task_lines("read three files", "three reads", Opening::FromJob);
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            ledger,
            RunPrompt::new(&prefix(), &mut PromptRecord::default()),
            &conversation,
            &[],
            &shape(),
        )
        .unwrap(),
    );
    advance(
        turn.call(
            Interrupt::None,
            ledger,
            &mut model,
            &BuildingPolicy::default(),
            Generating::Unwatched,
        )
        .unwrap(),
    )
}

#[test]
fn a_halt_inside_the_reads_starts_only_the_reads_before_it() {
    let calls = || vec![call("c1", "read"), call("c2", "read"), call("c3", "read")];
    let halt_at_two = |index: u32| match index {
        0 | 1 => Interrupt::None,
        _ => Interrupt::Cancel,
    };

    let mut serial = TestLedger::new();
    let turn = wave_of(&mut serial, calls());
    let mut ask = halt_at_two;
    let outcome = turn
        .execute_concurrent(
            Interrupt::None,
            &mut serial,
            &mut OneByOne(Placed::new()),
            &mut ask,
        )
        .unwrap();
    assert!(matches!(outcome, PhaseOutcome::Cancelled(_)));

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let mut ask = halt_at_two;
    let outcome = turn
        .execute_concurrent(
            Interrupt::None,
            &mut concurrent,
            &mut Placed::new(),
            &mut ask,
        )
        .unwrap();
    assert!(matches!(outcome, PhaseOutcome::Cancelled(_)));
    assert_eq!(concurrent.lines, serial.lines);
}

#[test]
fn a_steer_inside_the_reads_lands_where_the_serial_wave_writes_it() {
    let calls = || vec![call("c1", "read"), call("c2", "read"), call("c3", "read")];
    let steer_at_one = |index: u32| match index {
        1 => Interrupt::Steer {
            source: "person".into(),
            text: "only the tests".into(),
        },
        _ => Interrupt::None,
    };

    let mut serial = TestLedger::new();
    let turn = wave_of(&mut serial, calls());
    let mut ask = steer_at_one;
    advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut serial,
            &mut OneByOne(Placed::new()),
            &mut ask,
        )
        .unwrap(),
    );

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let mut ask = steer_at_one;
    advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut concurrent,
            &mut Placed::new(),
            &mut ask,
        )
        .unwrap(),
    );
    assert_eq!(concurrent.lines, serial.lines);
}

#[test]
fn a_call_line_carries_its_tools_registration() {
    let mut ledger = TestLedger::new();
    let mut lines = lines();
    let turn = waved(
        opened_on::<1>(&mut lines),
        &mut ledger,
        vec![call("c1", "read"), call("c2", "unknown")],
    );
    let recording = advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut ledger,
            &mut Placed::new(),
            &mut |_| Interrupt::None,
        )
        .unwrap(),
    );
    advance(recording.record(Interrupt::None, &mut ledger).unwrap());
    lines.barrier(&mut ledger).unwrap();
    let registered: Vec<(serde_json::Value, serde_json::Value)> = ledger
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|line| line["kind"] == "tool_called")
        .map(|line| {
            (
                line["data"]["effect"].clone(),
                line["data"]["render"].clone(),
            )
        })
        .collect();
    assert_eq!(
        registered,
        vec![
            (serde_json::json!("read"), serde_json::json!("generic")),
            (serde_json::Value::Null, serde_json::Value::Null),
        ],
        "a registered tool's line says what it was registered as; an unknown one says nothing"
    );
}

/// Which way a wave reaches its calls: each through all three stages in
/// turn, or the leading reads run at once.
#[derive(Debug, Clone, Copy)]
enum Path {
    OneAtATime,
    ReadsAtOnce,
}

/// A face that reads the driver's latest clock reading while it packages
/// each answer, the way the lane's exec face renders a result's clock
/// line (`crates/runtime/spec/Clock.lean` §8-53).
struct Stamping {
    placed: Placed,
    reading: crate::clock::ClockReading,
    path: Path,
    seen: Vec<TimeMs>,
}

impl ConcurrentInvoke for Stamping {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        match self.path {
            // A face that names no registration makes no call read-only.
            Path::OneAtATime => None,
            Path::ReadsAtOnce => self.placed.meta_of(call),
        }
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        self.placed.admit(call, t)
    }

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.placed.tool(ticket)
    }

    fn account(
        &mut self,
        call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.seen.extend(self.reading.latest());
        self.placed.account(call, ticket, answered)
    }
}

/// The reading a face stamps a result with is the moment that call's
/// `tool_result` records, on both ways through a wave: the turn reads
/// the answer's moment before the face packages the answer.
#[test]
fn a_stamp_the_face_reads_is_the_moment_its_answer_records() {
    for path in [Path::OneAtATime, Path::ReadsAtOnce] {
        let reading = crate::clock::ClockReading::default();
        let kept = reading.clone();
        let mut next = 100u64;
        let mut now = move || -> Result<TimeMs, AxError> {
            next = next.checked_add(1).unwrap();
            kept.keep(TimeMs::new(next));
            Ok(TimeMs::new(next))
        };
        let mut ledger = TestLedger::new();
        let mut lines = lines();
        let begun = Turn::begin(&mut lines, TimeMs::new(100), &mut now);
        let turn = waved(
            begun,
            &mut ledger,
            vec![call("c1", "read"), call("c2", "read")],
        );
        let mut face = Stamping {
            placed: Placed::new(),
            reading,
            path,
            seen: Vec::new(),
        };
        let recording = advance(
            turn.execute_concurrent(Interrupt::None, &mut ledger, &mut face, &mut |_| {
                Interrupt::None
            })
            .unwrap(),
        );
        advance(recording.record(Interrupt::None, &mut ledger).unwrap());
        lines.barrier(&mut ledger).unwrap();
        let answered: Vec<TimeMs> = ledger
            .lines
            .iter()
            .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
            .filter(|line| line["kind"] == "tool_result")
            .map(|line| TimeMs::new(line["t"].as_u64().unwrap()))
            .collect();
        assert_eq!(face.seen, answered, "{path:?}");
    }
}
