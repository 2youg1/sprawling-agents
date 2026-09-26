// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::disallowed_methods)]

use std::sync::{Condvar, Mutex};
use std::time::Duration;

use super::super::*;
use super::helpers::*;
use crate::conversation::Opening;
use kernel::{Effect, ToolOutcome};

const READS: u32 = 3;

/// Long enough that a thread the scheduler delays under a loaded build
/// still arrives; a serial wave waits it out on every read and fails.
const ARRIVAL_WAIT: Duration = Duration::from_secs(5);

/// Where the reads of one wave meet: each read waits until every read of
/// the wave has started, then notes how many had started while it was
/// still running. A read that saw them all overlapped every other read.
struct Rendezvous {
    started: Mutex<u32>,
    arrived: Condvar,
    fewest_seen: Mutex<u32>,
}

impl Rendezvous {
    fn new() -> Self {
        Self {
            started: Mutex::new(0),
            arrived: Condvar::new(),
            fewest_seen: Mutex::new(u32::MAX),
        }
    }

    fn read(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let mut started = self.started.lock().unwrap();
        *started += 1;
        self.arrived.notify_all();
        let (started, _) = self
            .arrived
            .wait_timeout_while(started, ARRIVAL_WAIT, |seen| *seen < READS)
            .unwrap();
        let mut fewest = self.fewest_seen.lock().unwrap();
        *fewest = (*fewest).min(*started);
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
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
            None,
        )
        .unwrap(),
    )
}

fn read(_call: &ToolCall) -> Result<ToolOutcome, AxError> {
    Ok(ToolOutcome {
        result: Payload::empty(),
        attachments: Vec::new(),
    })
}

fn effect_of(call: &ToolCall) -> Option<Effect> {
    match call.name.as_str() {
        "read" => Some(Effect::Read),
        _ => None,
    }
}

#[test]
fn three_reads_in_one_wave_are_in_flight_together_and_leave_the_serial_ledger() {
    let calls = || vec![call("c1", "read"), call("c2", "read"), call("c3", "read")];

    let mut serial = TestLedger::new();
    let turn = wave_of(&mut serial, calls());
    advance(
        turn.execute(Interrupt::None, &mut serial, &mut read, &mut |_| {
            Interrupt::None
        })
        .unwrap(),
    );

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let meeting = Rendezvous::new();
    let invoke = |call: &ToolCall| meeting.read(call);
    let tools = ConcurrentInvoke {
        invoke: &invoke,
        effect_of: &effect_of,
    };
    advance(
        turn.execute_concurrent(Interrupt::None, &mut concurrent, &tools, &mut |_| {
            Interrupt::None
        })
        .unwrap(),
    );

    assert_eq!(concurrent.lines, serial.lines);
    // In a serial wave the first read finishes before the second starts.
    assert_eq!(*meeting.fewest_seen.lock().unwrap(), READS);
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
        .execute(Interrupt::None, &mut serial, &mut read, &mut ask)
        .unwrap();
    assert!(matches!(outcome, PhaseOutcome::Cancelled(_)));

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let tools = ConcurrentInvoke {
        invoke: &read,
        effect_of: &effect_of,
    };
    let mut ask = halt_at_two;
    let outcome = turn
        .execute_concurrent(Interrupt::None, &mut concurrent, &tools, &mut ask)
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
        turn.execute(Interrupt::None, &mut serial, &mut read, &mut ask)
            .unwrap(),
    );

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let tools = ConcurrentInvoke {
        invoke: &read,
        effect_of: &effect_of,
    };
    let mut ask = steer_at_one;
    advance(
        turn.execute_concurrent(Interrupt::None, &mut concurrent, &tools, &mut ask)
            .unwrap(),
    );
    assert_eq!(concurrent.lines, serial.lines);
}
