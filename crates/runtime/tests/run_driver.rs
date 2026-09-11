// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The run driver owns one sequence: dispatch, turns, freeze. These tests
//! pin that sequence and the three ways a run can end, so a later change
//! to the loop has to argue with the event order rather than with prose.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{
    Address, AxError, B3Hash, BuildingPolicy, Completion, ContentBlock, EventDraft, EventRef,
    GENESIS_PREV, Ledger, Locator, Model, ModelRequest, ModelReturn, Payload, RunId, TimeMs,
    ToolCall, ToolName, ToolOutcome, chain_hash, message_payload,
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
        Err(AxError::failure(
            self.code,
            "read the model's reply",
            "not a tool name",
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
        shape: CallShape {
            model: "script".to_owned(),
            max_tokens: kernel::Ceiling::new(4096),
            effort: None,
            context_tokens: 0,
        },
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
        fence: None,
        invoke: &mut invoke,
        deltas: None,
    };

    let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert_eq!(
        ledger.kinds(),
        vec![
            "checkpoint_committed",
            "run_started",
            "prompt_assembled",
            "model_called",
            "model_returned",
            "tool_called",
            "tool_result",
            "prompt_assembled",
            "model_called",
            "model_returned",
            "handoff_written",
            "run_frozen",
        ]
    );
    assert!(matches!(frozen.completion(), Completion::Done(_)));
    assert_eq!(frozen.turns(), 2);
    // Dispatch takes two stamps, each turn one, and the freeze one more
    // with run_frozen derived from it: two ledger lines, one event.
    let stamps = ledger.stamps();
    assert_eq!(stamps[0], 0);
    assert_eq!(stamps[1], 1);
    assert_eq!(stamps[2], 2);
    assert_eq!(stamps[7], 3);
    assert_eq!(stamps[10], 4);
    assert_eq!(stamps[11], 5);
}

/// There is no ceiling to reach, so a run goes on until its
/// own work runs out. The script here is longer than the turn ceiling
/// this driver used to carry, and the run still ends by concluding
/// rather than by being cut off.
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
        fence: None,
        invoke: &mut invoke,
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
        fence: None,
        invoke: &mut invoke,
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
fn a_fence_runs_before_the_wave_and_carries_the_turns_stamp() {
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
    let mut fenced: Vec<u64> = Vec::new();
    let mut fence = |t: TimeMs| {
        fenced.push(t.value());
        Ok(Payload::empty())
    };
    {
        let mut hooks = RunHooks {
            now: &mut now,
            interrupt: &mut interrupt,
            fence: Some(&mut fence),
            invoke: &mut invoke,
            deltas: None,
        };
        let frozen = drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
        assert!(matches!(frozen.completion(), Completion::Done(_)));
    }
    // The fence goes up before *every* wave, including the last turn's
    // empty one: an unchanged wave still commits, because a chain that
    // rebuilds is worth more than a saved object.
    assert_eq!(fenced, vec![2, 3]);
    let kinds = ledger.kinds();
    assert_eq!(kinds[5], "checkpoint_committed");
    assert_eq!(kinds[6], "tool_called");
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
        fence: None,
        invoke: &mut invoke,
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
        fence: None,
        invoke: &mut invoke,
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
        fence: None,
        invoke: &mut invoke,
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
        fence: None,
        invoke: &mut invoke,
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
        fence: None,
        invoke: &mut invoke,
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
