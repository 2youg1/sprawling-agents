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
    // One steer, at the boundary before the second assembly.
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeAssemble { turn: 1 } => Interrupt::Steer {
            speaker: runtime::conversation::Speaker::Person,
            text: "measure it in metres".to_owned(),
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

/// The fourth boundary. A run whose last turn made no tool call would
/// otherwise conclude `Done` with nowhere left to stop it, and whatever
/// that turn handed down would start anyway. `BeforeSpawn` is the point
/// where a cancel arriving that late still lands.
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
    // The wave ran before the boundary was consulted: what the tools did
    // is on the ledger, and the cancel comes after it rather than
    // pretending the work never happened.
    assert!(kinds.iter().any(|kind| kind == "tool_result"));
}

/// **The defect this pins shipped and was found by driving a real
/// provider.** ModelScope's streaming tool calls repeat the tool name as
/// an empty string in every chunk after the first, so a dispatch died of
/// `E_WIRE_MISMATCH` on its first tool call. That code's carrier is
/// `Loadtime`, and the driver's error arm returned it without freezing —
/// so the run was dropped with only `run_started` behind it, `city_view`
/// reported it alive after a restart, and the page sent every later
/// message as a `steer` into a run that was never coming back.
///
/// A run always ends. The verdict is written before the diagnosis
/// travels, and both of them survive.
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
    // A loadtime code names no carrier event, so nothing stands between
    // the last turn and the verdict. `Provider` is the other half of the
    // table and the test below holds that arm.
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

/// A carrier failure that freezes the run names its own error on the
/// verdict, so the page can show why instead of inventing a carrier row.
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

/// The other half of the carrier table, held here so the two arms are
/// read side by side: a code that names an event writes that event
/// first, and the run freezes rather than propagating.
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

/// A provider answering 500 in a row is asked again on a backoff that
/// doubles (plus the run's jitter), each wait is the moment the history records, and a halt that
/// lands during a wait stops the run there. Before the wait hook the
/// loop asked again at once, recorded a wait it never took, and a root
/// run had no safe point inside the retry for a halt to reach.
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

/// A steer that arrives between two calls of a wave is written down
/// before the model reads it, the same as a steer at any other safe
/// point: the window that carries it is the model's, and a window
/// holding text the ledger never saw is a request nobody can account
/// for.
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
            speaker: runtime::conversation::Speaker::Person,
            text: "measure it in metres".to_owned(),
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
