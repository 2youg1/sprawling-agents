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
struct Placed {
    bench: ToolBench,
    next: u64,
}

impl Placed {
    fn new() -> Placed {
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

    /// The same calls one after another, through a closure: serial.
    fn one_by_one(&mut self, call: &ToolCall, t: TimeMs) -> Result<ToolOutcome, AxError> {
        let key = self.key();
        self.bench.invoke(call, &key, t).and_then(answer)
    }
}

fn answer(outcome: BenchOutcome) -> Result<ToolOutcome, AxError> {
    match outcome {
        BenchOutcome::Ran { outcome, .. } | BenchOutcome::Duplicate { outcome } => Ok(outcome),
        BenchOutcome::Refused { refusal } => Err(*refusal),
    }
}

impl ConcurrentInvoke for Placed {
    fn effect_of(&self, call: &ToolCall) -> Option<Effect> {
        self.bench
            .meta_of(&call.name)
            .map(|meta| meta.effect.clone())
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

fn wave_of(ledger: &mut TestLedger, calls: Vec<ToolCall>) -> Turn<ToolWave> {
    let mut model = OneShotModel { calls };
    let mut conversation = Conversation::new();
    conversation.push_task_lines("read three files", "three reads", Opening::FromJob);
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
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
    let mut bench = Placed::new();
    let outcome = turn
        .execute_concurrent(
            Interrupt::None,
            &mut serial,
            &mut |call: &ToolCall, t: TimeMs| bench.one_by_one(call, t),
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
    let mut bench = Placed::new();
    advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut serial,
            &mut |call: &ToolCall, t: TimeMs| bench.one_by_one(call, t),
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
