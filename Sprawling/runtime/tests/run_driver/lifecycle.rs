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

use kernel::ledger::chain_hash;
use kernel::model::message_payload;
use kernel::{
    Address, AxError, B3Hash, BuildingPolicy, Completion, ContentBlock, EventDraft, EventRef,
    GENESIS_PREV, Ledger, Locator, Model, ModelRequest, ModelReturn, Payload, RunId, TimeMs,
    ToolCall, ToolName, ToolOutcome,
};
use runtime::handoff::Handoff;
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use runtime::run::{Advance, Run, RunHooks, RunPlan, SafePoint, drive};
use runtime::turn::{CallShape, Interrupt};

use super::support::*;

#[test]
fn a_run_that_finishes_writes_dispatch_turns_and_freeze_in_that_order() {
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

    let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert_eq!(
        ledger.kinds(),
        vec![
            "checkpoint_committed",
            "run_started",
            "prompt_assembled",
            "prompt_shape_compared",
            "model_called",
            "model_returned",
            "tool_called",
            "tool_result",
            "prompt_shape_compared",
            "model_called",
            "model_returned",
            "handoff_written",
            "run_frozen",
        ]
    );
    assert!(matches!(frozen.completion(), Completion::Done(_)));
    assert_eq!(frozen.turns(), 2);
    // Dispatch takes two readings (0, 1). Turn one opens at 2, and the
    // assembly and shape lines ride that stamp; the model attempt is sent
    // at 3 and its reply is whole at 4; the tool call starts at 5 and
    // answers at 6. Turn two opens at 7 with its shape line; sent at 8,
    // whole at 9. The freeze reads 10 for handoff_written, and run_frozen
    // takes 11 from it: two ledger lines, one event.
    assert_eq!(
        ledger.stamps(),
        vec![0, 1, 2, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    );
    // What the turn wrote and what a reader asks agree: a line reads as
    // its own moment exactly when it is one of the four the turn waited for.
    for line in &ledger.lines {
        let record = kernel::EventRecord::parse_line(line).unwrap();
        let waited_for = matches!(
            record.kind(),
            kernel::EventKind::ModelCalled
                | kernel::EventKind::ModelReturned
                | kernel::EventKind::ToolCalled
                | kernel::EventKind::ToolResult
        );
        let expected = waited_for.then(|| record.t());
        assert_eq!(record.moment(), expected, "{:?}", record.kind());
    }
}

/// There is no ceiling to reach, so a run goes on until its
/// own work runs out. The script here is long, and the run still ends
/// by concluding rather than by being cut off.
/// The number a person enters on the provider form is what decides how
/// many times a failed call is made again, so one 503 does not end a
/// run.
#[test]
fn a_retriable_failure_is_made_again_up_to_the_number_the_person_set() {
    let attempts = std::rc::Rc::new(std::cell::RefCell::new(0));
    let mut ledger = RecordingLedger::new();
    let mut model = FlakyModel {
        failures_left: 2,
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
    let plan = RunPlan {
        retries: kernel::Retries::AtMost(2),
        ..plan()
    };

    let frozen = drive(plan, &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert!(matches!(frozen.completion(), Completion::Done(_)));
    assert_eq!(*attempts.borrow(), 3, "two failures, then the answer");
    assert_eq!(
        ledger
            .kinds()
            .iter()
            .filter(|k| *k == "watchdog_fired")
            .count(),
        2,
        "each retry is a record, not a silent repeat"
    );
}

/// The other side of the same number: a ceiling that is reached freezes
/// the run rather than asking a provider that keeps saying no.
#[test]
fn a_ceiling_that_is_reached_ends_the_run() {
    let attempts = std::rc::Rc::new(std::cell::RefCell::new(0));
    let mut ledger = RecordingLedger::new();
    let mut model = FlakyModel {
        failures_left: 9,
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
    let plan = RunPlan {
        retries: kernel::Retries::AtMost(1),
        ..plan()
    };

    let frozen = drive(plan, &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert!(matches!(frozen.completion(), Completion::Cancelled));
    assert_eq!(*attempts.borrow(), 2, "the first call, then the one retry");
}

#[test]
fn a_run_ends_when_its_work_runs_out_rather_than_at_a_ceiling() {
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: (0..26)
            .map(|turn| vec![call(&format!("t-{turn}"))])
            .collect(),
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

    assert!(matches!(frozen.completion(), Completion::Done(_)));
    assert_eq!(frozen.turns(), 27, "twenty-six waves, then the empty one");
    let kinds = ledger.kinds();
    assert_eq!(kinds[kinds.len() - 2], "handoff_written");
    assert_eq!(kinds[kinds.len() - 1], "run_frozen");
}

#[test]
fn a_cancel_at_a_safe_point_freezes_inside_the_interrupted_turn() {
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: vec![vec![call("t-1")]],
    };
    let mut now = counter();
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeCall { turn: 0 } => Interrupt::Cancel,
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
    let kinds = ledger.kinds();
    assert_eq!(kinds[kinds.len() - 3], "cancel_received");
    assert_eq!(kinds[kinds.len() - 2], "handoff_written");
    assert_eq!(kinds[kinds.len() - 1], "run_frozen");
    // The freeze belongs to the interrupted turn, so it carries that
    // turn's stamp rather than sampling a fresh one.
    let stamps = ledger.stamps();
    assert_eq!(stamps[2], 2);
    assert_eq!(stamps[stamps.len() - 2], 2);
    assert_eq!(stamps[stamps.len() - 1], 3);
}

#[test]
fn a_checkpoint_runs_before_the_wave_and_carries_the_turns_stamp() {
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
    let mut checkpointed: Vec<u64> = Vec::new();
    let mut checkpoint = |t: TimeMs| {
        checkpointed.push(t.value());
        Ok(Payload::empty())
    };
    {
        let mut hooks = RunHooks {
            now: &mut now,
            monotonic_us: &mut || 0,
            interrupt: &mut interrupt,
            checkpoint: Some(&mut checkpoint),
            writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
            invoke: &mut invoke,
            wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
            deltas: None,
        };
        let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
        assert!(matches!(frozen.completion(), Completion::Done(_)));
    }
    // The checkpoint goes up before the wave that calls something, and again
    // before the closing turn's empty wave: that one has nothing to come
    // back from, but it is what carries the first wave's writes into a
    // commit.
    assert_eq!(checkpointed, vec![2, 7]);
    let kinds = ledger.kinds();
    assert_eq!(kinds[6], "checkpoint_committed");
    assert_eq!(kinds[7], "tool_called");
}

/// A known-shape checkpoint refusal leaves a structured, byte-free record
/// before the run freezes cancelled through the driver's ordinary carrier path.
#[test]
fn a_refused_checkpoint_records_its_scope_and_findings_before_the_run_freezes() {
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
    let report = kernel::event::record::SecretEgressBlocked::checkpoint(
        vec!["lab".to_owned()],
        vec![kernel::event::record::CheckpointFinding {
            path: "lab/SPEC.md".to_owned(),
            offset: 9,
            len: 37,
            line: 1,
            reason: "anthropic".to_owned(),
        }],
    );
    let mut checkpoint = |_: TimeMs| {
        Err(runtime::CheckpointFailure::from(
            storage::StorageError::SecretEgress(report.clone()),
        ))
    };
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: Some(&mut checkpoint),
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };
    let outcome = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff());
    let frozen = outcome.unwrap();
    assert!(matches!(frozen.completion(), Completion::Cancelled));
    let blocked = ledger
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .find(|line| line["kind"] == "secret_egress_blocked")
        .expect("the refusal has its own queryable event");
    assert_eq!(blocked["data"]["code"], "E_SECRET_EGRESS");
    assert_eq!(blocked["data"]["scope"], serde_json::json!(["lab"]));
    assert_eq!(blocked["data"]["withheld"][0]["line"], 1);
    assert_eq!(blocked["data"]["withheld"][0]["reason"], "anthropic");
    assert_eq!(
        ledger.kinds().last().map(String::as_str),
        Some("run_frozen")
    );
}

#[test]
fn a_run_that_calls_nothing_puts_up_no_checkpoint() {
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: Vec::new(),
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |_: &ToolCall, _: TimeMs| {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    let mut checkpointed: Vec<u64> = Vec::new();
    let mut checkpoint = |t: TimeMs| {
        checkpointed.push(t.value());
        Ok(Payload::empty())
    };
    {
        let mut hooks = RunHooks {
            now: &mut now,
            monotonic_us: &mut || 0,
            interrupt: &mut interrupt,
            checkpoint: Some(&mut checkpoint),
            writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
            invoke: &mut invoke,
            wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
            deltas: None,
        };
        let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
        assert!(matches!(frozen.completion(), Completion::Done(_)));
    }
    // Nothing ran and nothing will: the tree is the one the run opened on.
    assert_eq!(checkpointed, Vec::<u64>::new());
}

/// A wave whose every call only reads changes no file: it needs no checkpoint
/// before it, and the closing turn after it has no writes to carry.
#[test]
fn a_read_only_wave_puts_up_no_checkpoint() {
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
    let mut checkpointed: Vec<u64> = Vec::new();
    let mut checkpoint = |t: TimeMs| {
        checkpointed.push(t.value());
        Ok(Payload::empty())
    };
    {
        let mut hooks = RunHooks {
            now: &mut now,
            monotonic_us: &mut || 0,
            interrupt: &mut interrupt,
            checkpoint: Some(&mut checkpoint),
            writes: &|_: &kernel::ToolCall| kernel::Writes::Nothing,
            invoke: &mut invoke,
            wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
            deltas: None,
        };
        let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
        assert!(matches!(frozen.completion(), Completion::Done(_)));
    }
    assert_eq!(checkpointed, Vec::<u64>::new());
}
