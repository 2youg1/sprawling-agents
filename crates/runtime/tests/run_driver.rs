// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "test code: this driver names the outcomes it asserts on               and treats the rest as one arm, rather than modelling an               enum the assertion does not reach"
)]

//! The run driver owns one sequence: dispatch, turns, freeze. These tests
//! pin that sequence and the three ways a run can end, so a later change
//! to the loop has to argue with the event order rather than with prose.

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

struct RecordingLedger {
    lines: Vec<Vec<u8>>,
    next: kernel::Seq,
    prev: B3Hash,
}

impl RecordingLedger {
    fn new() -> Self {
        RecordingLedger {
            lines: Vec::new(),
            next: kernel::Seq::FIRST,
            prev: GENESIS_PREV,
        }
    }

    fn field(&self, key: &str) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_slice(line).unwrap();
                value[key].to_string().trim_matches('"').to_owned()
            })
            .collect()
    }

    fn kinds(&self) -> Vec<String> {
        self.field("kind")
    }

    fn stamps(&self) -> Vec<u64> {
        self.lines
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_slice(line).unwrap();
                value["t"].as_u64().unwrap()
            })
            .collect()
    }
}

impl Ledger for RecordingLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let record = kernel::EventRecord::from_draft(draft, self.next, self.prev);
        let line = record.canonical_line()?;
        self.prev = chain_hash(&line);
        self.next = self.next.next()?;
        let echo = record.to_ref();
        self.lines.push(line);
        Ok(echo)
    }
}

/// One tool call on the first turn, nothing after: the shortest script
/// that still exercises a wave.
struct ScriptedModel {
    waves: Vec<Vec<ToolCall>>,
    /// Every user message this model was shown, in order, so a test can
    /// ask what actually reached the window rather than what was
    /// recorded about it.
    seen: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Model for ScriptedModel {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.seen
            .borrow_mut()
            .push(format!("{:?}", req.chat.messages));
        let calls = if self.waves.is_empty() {
            Vec::new()
        } else {
            self.waves.remove(0)
        };
        Ok(ModelReturn::bare(
            message_payload(&[ContentBlock::Text {
                text: "working".to_owned(),
            }])
            .unwrap(),
            calls,
        ))
    }
}

/// A model whose every call fails with one code.
///
/// The two codes this fixture is used with are the two halves of the
/// carrier table: one that names an event to write, and one the table
/// calls loadtime. A run has to end under both.
struct FailingModel {
    code: kernel::AxCode,
}

impl Model for FailingModel {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Err(
            AxError::failure(self.code, "read the model's reply", "not a tool name")
                .with_recovery("name a tool the run registered, or register this one"),
        )
    }
}

/// A model that fails for a reason worth retrying, a stated number of
/// times, and then answers.
struct FlakyModel {
    failures_left: u32,
    calls: std::rc::Rc<std::cell::RefCell<u32>>,
}

impl Model for FlakyModel {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        let spent = *self.calls.borrow();
        *self.calls.borrow_mut() = spent.saturating_add(1);
        if self.failures_left > 0 {
            self.failures_left = self.failures_left.saturating_sub(1);
            return Err(AxError::failure(
                kernel::AxCode::Provider,
                "call the model",
                "the provider answered 503",
            )
            .retriable()
            .with_recovery("wait for the provider and send the turn again"));
        }
        Ok(ModelReturn::bare(
            message_payload(&[ContentBlock::Text {
                text: "working".to_owned(),
            }])
            .unwrap(),
            Vec::new(),
        ))
    }
}

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("status").unwrap(),
        args: Payload::empty(),
    }
}

fn plan() -> RunPlan {
    let addr = Address::parse("lab/room1").unwrap();
    RunPlan {
        run: RunId::from_bytes([7; 16]),
        who: "resident".to_owned(),
        addr: addr.clone(),
        task: "close the loop".to_owned(),
        goal: "one turn, then stop".to_owned(),
        opening: runtime::Opening::FromJob,
        job: Locator::parse(&format!("file:{}/JOB.md@{}", addr.as_str(), "a".repeat(40))).unwrap(),
        parent: None,
        predecessor: None,
        dispatched_by: kernel::event::Who::Person,
        run_policy: kernel::RunPolicy::of(kernel::Mode::Work),
        naming: None,
        inherited: Vec::new(),
        shape: CallShape {
            model: "script".to_owned(),
            max_tokens: kernel::Ceiling::new(4096),
            effort: None,
            context_tokens: 0,
        },
        second_threshold: None,
        context: runtime::ContextReading::default(),
        prefix: FrozenPrefix::assemble(
            FrozenSegment::new(SegmentSlot::City, b"city".to_vec()),
            FrozenSegment::new(SegmentSlot::Building, b"building".to_vec()),
            FrozenSegment::new(SegmentSlot::Resident, b"resident".to_vec()),
            FrozenSegment::new(SegmentSlot::Run, b"run".to_vec()),
        )
        .unwrap(),
        policy: BuildingPolicy::default(),
        tools: Vec::new(),
        skills: Vec::new(),
        retries: kernel::Retries::UntilHalted,
    }
}

fn handoff() -> Handoff {
    Handoff::new(
        vec![Locator::parse(&format!("file:lab/room1/JOB.md@{}", "a".repeat(40))).unwrap()],
        "driver test".to_owned(),
        "see roadmap".to_owned(),
        "scripted world".to_owned(),
        "resume from the job file".to_owned(),
    )
    .unwrap()
}

/// A counter clock: the same closure citysim uses, so the assertions here
/// and the simulator's byte fixtures are talking about one discipline.
fn counter() -> impl FnMut() -> Result<TimeMs, AxError> {
    let mut tick: u64 = 0;
    move || {
        let now = TimeMs::new(tick);
        tick = tick.saturating_add(1);
        Ok(now)
    }
}

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
            source: "user".to_owned(),
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
            source: "user".to_owned(),
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

const READS: u32 = 3;

/// Long enough that a thread the scheduler delays under a loaded build
/// still arrives; a serial wave waits it out on every read and fails.
const ARRIVAL_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Where the reads of one wave meet: each read waits until every read of
/// the wave has started, then notes how many had started while it was
/// still running. A read that saw them all overlapped every other read.
struct Meeting {
    started: std::sync::Mutex<u32>,
    arrived: std::sync::Condvar,
    fewest_seen: std::sync::Mutex<u32>,
}

/// A read-only tool; with a meeting, each call waits at it.
struct MeetingRead {
    meta: kernel::ToolMeta,
    meeting: Option<std::sync::Arc<Meeting>>,
}

impl kernel::Tool for MeetingRead {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.meta
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if let Some(meeting) = &self.meeting {
            let mut started = meeting.started.lock().unwrap();
            *started = started.saturating_add(1);
            meeting.arrived.notify_all();
            let (started, _) = meeting
                .arrived
                .wait_timeout_while(started, ARRIVAL_WAIT, |seen| *seen < READS)
                .unwrap();
            let mut fewest = meeting.fewest_seen.lock().unwrap();
            *fewest = (*fewest).min(*started);
        }
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

/// The bench's three stages, with each call's key placed by its
/// position in the run, as the lane places it. It stays because runtime
/// cannot depend on sprawling, where the served city's `Placing` lives;
/// that one is judged in sprawling's `driving::tests::placing`.
struct Placed {
    bench: runtime::bench::ToolBench,
    next: u64,
}

impl Placed {
    fn with(meeting: Option<std::sync::Arc<Meeting>>) -> Placed {
        Placed::of(Box::new(MeetingRead {
            meta: read_meta(),
            meeting,
        }))
    }

    fn of(read: Box<dyn kernel::Tool>) -> Placed {
        let domain = kernel::WriteDomain::new(vec![Address::parse("lab").unwrap()]).unwrap();
        let mut bench = runtime::bench::ToolBench::new(domain);
        bench.register(read).unwrap();
        Placed { bench, next: 0 }
    }

    fn key(&mut self, call: &ToolCall) -> kernel::IdemKey {
        let at = self.next;
        self.next = at.saturating_add(1);
        kernel::IdemKey::derive(
            &RunId::from_bytes([7; 16]),
            kernel::Seq::new(at),
            call.id.as_bytes(),
        )
    }
}

/// The serial reference: each call answered where it is admitted, so no
/// read starts early, with each tool's registration named as the lane's
/// own face names it - the ledgers then differ only if the order or the
/// payloads do.
struct OneByOne(Placed);

impl runtime::ConcurrentInvoke for OneByOne {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.0.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> runtime::Admitted {
        let key = self.0.key(call);
        runtime::Admitted::Answered(self.0.bench.invoke(call, &key, t).and_then(answer))
    }

    fn tool(&self, ticket: &runtime::bench::Ticket) -> Result<&dyn kernel::Tool, AxError> {
        self.0.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        ticket: runtime::bench::Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.0.bench.account(ticket, answered).and_then(answer)
    }
}

fn read_meta() -> kernel::ToolMeta {
    kernel::ToolMeta {
        name: ToolName::parse("read").unwrap(),
        disclosure: "reads a file".to_owned(),
        params: Payload::empty(),
        effect: kernel::Effect::Read,
        cost_tier: kernel::CostTier::Free,
        timeout: None,
        render: kernel::RenderIntent::Generic,
        temporal: kernel::Temporal::Timeless,
    }
}

fn answer(outcome: runtime::bench::BenchOutcome) -> Result<ToolOutcome, AxError> {
    match outcome {
        runtime::bench::BenchOutcome::Ran { outcome, .. }
        | runtime::bench::BenchOutcome::Duplicate { outcome } => Ok(outcome),
        runtime::bench::BenchOutcome::Refused { refusal } => Err(*refusal),
    }
}

impl runtime::ConcurrentInvoke for Placed {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> runtime::Admitted {
        let key = self.key(call);
        match self.bench.clear(call, &key, t) {
            Ok(runtime::bench::Clearance::Cleared(ticket)) => runtime::Admitted::Cleared(ticket),
            Ok(runtime::bench::Clearance::Answered(outcome)) => {
                runtime::Admitted::Answered(answer(outcome))
            }
            Err(refused) => runtime::Admitted::Answered(Err(refused)),
        }
    }

    fn ahead(&self, call: &ToolCall) -> Option<&dyn kernel::Tool> {
        self.bench.tool_named(&call.name)
    }

    fn tool(&self, ticket: &runtime::bench::Ticket) -> Result<&dyn kernel::Tool, AxError> {
        self.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        ticket: runtime::bench::Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.bench.account(ticket, answered).and_then(answer)
    }
}

fn three_reads_driven(invoke: &mut dyn runtime::ConcurrentInvoke) -> RecordingLedger {
    let read = |id: &str| ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("read").unwrap(),
        args: Payload::empty(),
    };
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: vec![vec![read("r1"), read("r2"), read("r3")]],
    };
    // A stopped clock: reads that overlap take their readings in another
    // order than reads made one after another, and what the two ledgers
    // must share is the order and the payloads of their lines.
    let mut now = || -> Result<TimeMs, AxError> { Ok(TimeMs::new(0)) };
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut hooks = RunHooks {
        now: &mut now,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };
    drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
    ledger
}

/// The production driver runs a wave's leading reads at once, and the
/// ledger it leaves is the one the same calls leave one after another.
#[test]
fn three_reads_a_run_makes_in_one_wave_are_in_flight_together_and_leave_the_serial_ledger() {
    let serial = three_reads_driven(&mut OneByOne(Placed::with(None)));

    let meeting = std::sync::Arc::new(Meeting {
        started: std::sync::Mutex::new(0),
        arrived: std::sync::Condvar::new(),
        fewest_seen: std::sync::Mutex::new(u32::MAX),
    });
    let concurrent = three_reads_driven(&mut Placed::with(Some(meeting.clone())));

    assert_eq!(concurrent.lines, serial.lines);
    // In a serial wave the first read finishes before the second starts.
    assert_eq!(*meeting.fewest_seen.lock().unwrap(), READS);
}

#[test]
fn a_steer_after_assembly_leaves_the_sent_request_untouched() {
    let mut ledger = RecordingLedger::new();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut model = ScriptedModel {
        seen: std::rc::Rc::clone(&seen),
        waves: vec![vec![call("t-1")]],
    };
    let mut now = counter();
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeCall { turn: 0 } => Interrupt::Steer {
            source: "user".to_owned(),
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
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };

    drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    let seen = seen.borrow();
    // The breakpoint is a request-side annotation, so a message prints
    // the same in every request that carries it: every byte before the
    // closing bracket is the request already sent.
    let first = seen[0].strip_suffix(']').unwrap();
    assert!(
        seen[1].starts_with(first),
        "the second request extends the first:\n{}\n{}",
        seen[0],
        seen[1]
    );
    assert!(
        seen[1].ends_with("Text { text: \"user: measure it in metres\" }] }]"),
        "the steer lands after the result it arrived during: {}",
        seen[1]
    );
}

/// How long the generating model writes text after it hands its read
/// over, and how long that read takes to answer.
const WRITING: std::time::Duration = std::time::Duration::from_millis(500);
const READING: std::time::Duration = std::time::Duration::from_millis(500);

/// A read-only tool that takes `READING` to answer.
struct SlowRead;

impl kernel::Tool for SlowRead {
    fn meta(&self) -> &kernel::ToolMeta {
        static META: std::sync::OnceLock<kernel::ToolMeta> = std::sync::OnceLock::new();
        META.get_or_init(read_meta)
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        std::thread::sleep(READING);
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

/// A streaming model that hands each call of its wave over as the call
/// completes, then writes text for `WRITING` before its answer settles.
struct GeneratingModel {
    waves: Vec<Vec<ToolCall>>,
}

impl Model for GeneratingModel {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.call_speculating(req, &mut |_: &kernel::Increment| {}, &mut |_: &ToolCall| {})
    }

    fn call_speculating(
        &mut self,
        _req: &ModelRequest,
        _onto: kernel::Increments<'_>,
        early: kernel::EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        let calls = if self.waves.is_empty() {
            Vec::new()
        } else {
            self.waves.remove(0)
        };
        calls.iter().for_each(|call| early(call));
        if !calls.is_empty() {
            std::thread::sleep(WRITING);
        }
        Ok(ModelReturn::bare(
            message_payload(&[ContentBlock::Text {
                text: "reading".to_owned(),
            }])
            .unwrap(),
            calls,
        ))
    }
}

fn one_read_while_generating(
    invoke: &mut dyn runtime::ConcurrentInvoke,
) -> (RecordingLedger, std::time::Duration) {
    let mut ledger = RecordingLedger::new();
    let mut model = GeneratingModel {
        waves: vec![vec![ToolCall {
            id: "r1".to_owned(),
            name: ToolName::parse("read").unwrap(),
            args: Payload::empty(),
        }]],
    };
    let mut now = counter();
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut hooks = RunHooks {
        now: &mut now,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };
    let began = std::time::Instant::now();
    drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
    (ledger, began.elapsed())
}

/// A read the model hands over before it finishes writing runs while it
/// writes: the turn takes about the longer of the two, not their sum,
/// and the ledger is the one the same read leaves when it runs after the
/// answer settles.
#[test]
fn a_read_handed_over_while_the_model_writes_runs_during_the_writing() {
    let (serial, _) = one_read_while_generating(&mut OneByOne(Placed::of(Box::new(SlowRead))));

    let (speculated, took) = one_read_while_generating(&mut Placed::of(Box::new(SlowRead)));

    assert!(
        took < WRITING + READING,
        "the read waited for the model to finish writing: the run took {took:?}"
    );
    assert_eq!(speculated.lines, serial.lines);
}
