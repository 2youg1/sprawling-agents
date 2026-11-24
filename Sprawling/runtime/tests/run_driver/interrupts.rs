// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "test code: this driver names the outcomes it asserts on               and treats the rest as one arm, rather than modelling an               enum the assertion does not reach"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::redundant_closure,
    clippy::disallowed_methods,
    reason = "test code"
)]

use kernel::{Completion, Payload, TimeMs, ToolCall, ToolOutcome};
use runtime::run::{Advance, Run, RunHooks, SafePoint, drive};
use runtime::turn::Interrupt;

use super::support::*;

#[test]
fn advance_reports_each_turn_so_a_caller_can_stop_between_them() {
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: vec![vec![call("t-1")]],
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    let mut run = Run::dispatch(plan(), &mut ledger, &mut hooks).unwrap();
    assert!(matches!(
        run.advance(&mut ledger, &mut model, &mut hooks).unwrap(),
        Advance::Turned
    ));
    let ending = run.advance(&mut ledger, &mut model, &mut hooks).unwrap();
    let Advance::Concluded(completion) = ending else {
        panic!("the second turn makes no calls, so the run concludes");
    };
    assert!(matches!(completion, Completion::Done(_)));
    let frozen = run
        .freeze(&mut ledger, &handoff(), completion, &mut hooks)
        .unwrap();
    assert_eq!(frozen.turns(), 2);
}

#[test]
fn a_steer_at_a_safe_point_reaches_the_next_window_and_not_only_the_ledger() {
    let mut ledger = RecordingLedger::new();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut model = ScriptedModel {
        seen: std::rc::Rc::clone(&seen),
        waves: vec![vec![call("t-1")]],
    };
    let mut now = counter();
    let picture = kernel::ImageRef {
        locator: kernel::Locator::parse(&format!("cas:b3-{}", "cd".repeat(32))).unwrap(),
        media_type: kernel::ImageType::Png,
        width: 17,
        height: 5,
    };
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeAssemble { turn: 1 } => Interrupt::Steer {
            pictures: vec![picture.clone()],
            speaker: runtime::conversation::Speaker::Person,
            text: "measure it in metres".to_owned(),
            request: None,
        },
        _ => Interrupt::None,
    };
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert!(
        ledger.kinds().contains(&"steer_received".to_owned()),
        "the arrival is recorded"
    );
    let windows = seen.borrow();
    assert_eq!(windows.len(), 2, "two turns, two windows");
    assert!(!windows[0].contains(&picture.locator.to_string()));
    assert!(windows[1].contains(&picture.locator.to_string()));
    assert!(
        !windows[0].contains("measure it in metres"),
        "a steer never rewrites a request already assembled"
    );
    assert!(
        windows[1].contains("measure it in metres"),
        "and the next assembly carries it: {}",
        windows[1]
    );
    assert!(windows[1].contains("user"), "attributed to whoever sent it");
}

#[test]
fn a_cancel_after_the_wave_stops_the_run_before_anything_it_handed_down_starts() {
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: vec![vec![call("t-1")]],
    };
    let mut now = counter();
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeSpawn { turn: 0 } => Interrupt::Cancel,
        _ => Interrupt::None,
    };
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert!(matches!(frozen.completion(), Completion::Cancelled));
    assert_eq!(
        frozen.turns(),
        0,
        "a turn cancelled at its close did not count"
    );
    let kinds = ledger.kinds();
    assert_eq!(kinds[kinds.len() - 3], "cancel_received");
    assert!(kinds.iter().any(|kind| kind == "tool_result"));
}

#[test]
fn a_run_that_dies_of_a_loadtime_failure_still_writes_its_verdict() {
    let mut ledger = RecordingLedger::new();
    let mut model = FailingModel {
        code: kernel::AxCode::WireMismatch,
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    let outcome = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff());

    let Err(err) = outcome else {
        panic!("the diagnosis still reaches the caller");
    };
    assert_eq!(*err.code(), kernel::AxCode::WireMismatch);
    let kinds = ledger.kinds();
    assert_eq!(
        kinds.last().map(String::as_str),
        Some("run_frozen"),
        "a run that started must not be left without a verdict: {kinds:?}"
    );
    assert_eq!(
        kinds[kinds.len() - 2],
        "handoff_written",
        "the one exit writes both of its lines: {kinds:?}"
    );
    assert!(
        !kinds.iter().any(|kind| kind == "provider_degraded"),
        "a loadtime code invents no carrier: {kinds:?}"
    );
    let frozen = ledger
        .lines
        .iter()
        .rev()
        .find_map(|line| {
            let value: serde_json::Value = serde_json::from_slice(line).unwrap();
            (value["kind"] == "run_frozen").then_some(value)
        })
        .expect("the verdict is on the ledger");
    assert_eq!(
        frozen["data"]["failure"]["code"],
        serde_json::json!("E_WIRE_MISMATCH")
    );
}

#[test]
fn a_carrier_failure_writes_its_reason_on_the_verdict() {
    let mut ledger = RecordingLedger::new();
    let mut model = FailingModel {
        code: kernel::AxCode::Provider,
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
    assert!(matches!(frozen.completion(), Completion::Cancelled));
    let last: serde_json::Value =
        serde_json::from_slice(ledger.lines.last().expect("the verdict is the last line")).unwrap();
    assert_eq!(
        last["data"]["failure"]["code"],
        serde_json::json!("E_PROVIDER")
    );
}

#[test]
fn a_provider_failure_writes_its_carrier_and_then_the_verdict() {
    let mut ledger = RecordingLedger::new();
    let mut model = FailingModel {
        code: kernel::AxCode::Provider,
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff())
        .expect("a carrier code ends the run rather than escaping it");

    assert!(matches!(frozen.completion(), Completion::Cancelled));
    let kinds = ledger.kinds();
    assert_eq!(kinds[kinds.len() - 3], "provider_degraded");
    assert_eq!(kinds[kinds.len() - 2], "handoff_written");
    assert_eq!(kinds.last().map(String::as_str), Some("run_frozen"));
}

#[test]
fn failures_in_a_row_back_off_and_a_halt_during_the_wait_stops_the_run() {
    let attempts = std::rc::Rc::new(std::cell::RefCell::new(0));
    let mut ledger = RecordingLedger::new();
    let mut model = FlakyModel {
        failures_left: 5,
        calls: std::rc::Rc::clone(&attempts),
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut waited: Vec<u64> = Vec::new();
    let mut wait = |until: TimeMs| {
        waited.push(until.value());
        if waited.len() < 3 {
            runtime::NextCall::Allowed
        } else {
            runtime::NextCall::Halted
        }
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut wait,
        deltas: None,
    };

    let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    let fired: Vec<(u64, u64)> = ledger
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|line| line["kind"] == "watchdog_fired")
        .map(|line| {
            (
                line["t"].as_u64().unwrap(),
                line["data"]["until_ms"].as_u64().unwrap(),
            )
        })
        .collect();
    let backoffs: Vec<u64> = fired.iter().map(|(t, until)| until - t).collect();
    let untils: Vec<u64> = fired.iter().map(|(_, until)| *until).collect();
    assert!(
        backoffs
            .iter()
            .zip([500, 1_000, 2_000])
            .all(|(wait, base)| (base..=base + base / 2).contains(wait))
            && backoffs.len() == 3,
        "the wait doubles, plus at most half again of jitter: {backoffs:?}"
    );
    assert_eq!(waited, untils, "the run waits for the moment it records");
    assert_eq!(*attempts.borrow(), 3, "no call goes out after the halt");
    assert!(matches!(frozen.completion(), Completion::Cancelled));
}

#[test]
fn a_steer_inside_a_tool_wave_is_recorded_before_the_model_reads_it() {
    let mut ledger = RecordingLedger::new();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut model = ScriptedModel {
        seen: std::rc::Rc::clone(&seen),
        waves: vec![vec![call("t-1"), call("t-2")]],
    };
    let mut now = counter();
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeToolCall { turn: 0, call: 1 } => Interrupt::Steer {
            pictures: Vec::new(),
            speaker: runtime::conversation::Speaker::Person,
            text: "measure it in metres".to_owned(),
            request: None,
        },
        _ => Interrupt::None,
    };
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    let kinds = ledger.kinds();
    let steered = kinds
        .iter()
        .position(|kind| kind == "steer_received")
        .unwrap_or_else(|| panic!("the steer inside the wave is recorded: {kinds:?}"));
    let called_again = kinds
        .iter()
        .rposition(|kind| kind == "model_called")
        .unwrap();
    assert!(
        steered < called_again,
        "recorded before the call that carries it: {kinds:?}"
    );
    assert!(
        seen.borrow()[1].contains("measure it in metres"),
        "and the next assembly carries it"
    );
}
