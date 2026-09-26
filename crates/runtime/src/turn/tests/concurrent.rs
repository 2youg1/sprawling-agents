// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::disallowed_methods)]

use std::time::{Duration, Instant};

use super::super::*;
use super::helpers::*;
use crate::window::Opening;
use kernel::{Effect, ToolOutcome};

const READ_MS: u64 = 50;

fn call(id: &str, tool: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: kernel::ToolName::parse(tool).unwrap(),
        args: Payload::empty(),
    }
}

fn wave_of(ledger: &mut TestLedger, calls: Vec<ToolCall>) -> Turn<ToolWave> {
    let mut model = OneShotModel { calls };
    let mut window = Window::new();
    window.push_task_lines("read three files", "three reads", Opening::FromJob);
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(Interrupt::None, ledger, &prefix(), &window, &[], &shape())
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

fn slow_read(_call: &ToolCall) -> Result<ToolOutcome, AxError> {
    std::thread::sleep(Duration::from_millis(READ_MS));
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
fn three_reads_in_one_wave_take_the_time_of_one_and_leave_the_serial_ledger() {
    let calls = || vec![call("c1", "read"), call("c2", "read"), call("c3", "read")];

    let mut serial = TestLedger::new();
    let turn = wave_of(&mut serial, calls());
    advance(
        turn.execute(Interrupt::None, &mut serial, &mut slow_read, &mut |_| {
            NextCall::Allowed
        })
        .unwrap(),
    );

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let tools = ConcurrentInvoke {
        invoke: &slow_read,
        effect_of: &effect_of,
    };
    let started = Instant::now();
    advance(
        turn.execute_concurrent(Interrupt::None, &mut concurrent, &tools, &mut |_| {
            NextCall::Allowed
        })
        .unwrap(),
    );
    let took = started.elapsed();

    assert_eq!(concurrent.lines, serial.lines);
    // A serial wave spends three read times.
    assert!(
        took <= Duration::from_millis(READ_MS + 10),
        "the wave took {took:?}"
    );
}

#[test]
fn a_halt_inside_the_reads_starts_only_the_reads_before_it() {
    let calls = || vec![call("c1", "read"), call("c2", "read"), call("c3", "read")];
    let halt_at_two = |index: u32| match index {
        0 | 1 => NextCall::Allowed,
        _ => NextCall::Halted,
    };

    let mut serial = TestLedger::new();
    let turn = wave_of(&mut serial, calls());
    let mut ask = halt_at_two;
    let outcome = turn
        .execute(Interrupt::None, &mut serial, &mut slow_read, &mut ask)
        .unwrap();
    assert!(matches!(outcome, PhaseOutcome::Cancelled(_)));

    let mut concurrent = TestLedger::new();
    let turn = wave_of(&mut concurrent, calls());
    let tools = ConcurrentInvoke {
        invoke: &slow_read,
        effect_of: &effect_of,
    };
    let mut ask = halt_at_two;
    let outcome = turn
        .execute_concurrent(Interrupt::None, &mut concurrent, &tools, &mut ask)
        .unwrap();
    assert!(matches!(outcome, PhaseOutcome::Cancelled(_)));
    assert_eq!(concurrent.lines, serial.lines);
}
