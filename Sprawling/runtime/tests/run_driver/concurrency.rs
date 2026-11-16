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

use super::meeting::*;

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

/// A policy change that reaches the run before its call is taken at the
/// wave, and the model reads of it after that wave's results: every byte
/// the first request sent is the start of the second (runtime §8-62).
#[test]
fn a_policy_change_is_told_after_the_wave_and_leaves_the_sent_request_untouched() {
    let mut ledger = RecordingLedger::new();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut model = ScriptedModel {
        seen: std::rc::Rc::clone(&seen),
        waves: vec![vec![call("t-1")]],
    };
    let mut now = counter();
    let tighter = kernel::RunPolicy {
        write: kernel::WriteLimit::Create,
        ..kernel::RunPolicy::of(kernel::Mode::Work)
    };
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeCall { turn: 0 } => Interrupt::Policy { policy: tighter },
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

    let seen = seen.borrow();
    let first = seen[0].strip_suffix(']').unwrap();
    assert!(
        seen[1].starts_with(first),
        "the second request extends the first:
{}
{}",
        seen[0],
        seen[1]
    );
    assert!(
        seen[1].ends_with(
            "Text { text: \"city: The User changed this run's policy; from the next tool call \
             on: mode work, write create, admission standing, landing ordinary.\" }] }]"
        ),
        "the note lands after the results of the wave it was taken at: {}",
        seen[1]
    );
}

/// A policy change that reaches a running run gates the next wave's
/// edit: the tool reads the run's policy cell at the call, not the write
/// limit the run was dispatched under (runtime §8-62).
#[test]
fn a_policy_changed_mid_run_gates_the_next_waves_edit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let room = Address::parse("lab/room1").unwrap();
    std::fs::create_dir_all(root.join("lab").join("room1")).unwrap();
    let kept = root.join("lab").join("room1").join("kept.md");
    std::fs::write(
        &kept,
        "the User's own
",
    )
    .unwrap();
    let plan = plan();
    let edit = runtime::EditTool::new(
        root,
        room.clone(),
        kernel::WriteDomain::new(vec![room]).unwrap(),
        runtime::EditDesk {
            policy: plan.run_policy.reader(),
            seen: runtime::ReadStamps::default(),
            locks: runtime::PathLocks::default(),
        },
    )
    .unwrap();
    let mut args = serde_json::Map::new();
    for (key, value) in [
        ("file_path", "lab/room1/kept.md".to_owned()),
        ("old_string", "own".to_owned()),
        ("new_string", "changed".to_owned()),
    ] {
        args.insert(key.to_owned(), serde_json::Value::String(value));
    }
    let editing = ToolCall {
        id: "t-1".to_owned(),
        name: ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    };
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: vec![vec![editing]],
    };
    let mut now = counter();
    let tighter = kernel::RunPolicy {
        write: kernel::WriteLimit::Create,
        ..kernel::RunPolicy::of(kernel::Mode::Work)
    };
    let mut interrupt = |point: SafePoint| match point {
        SafePoint::BeforeCall { turn: 0 } => Interrupt::Policy { policy: tighter },
        _ => Interrupt::None,
    };
    let mut invoke = |call: &ToolCall, _: TimeMs| kernel::Tool::invoke(&edit, call);
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

    drive(plan, &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();

    assert_eq!(
        std::fs::read_to_string(&kept).unwrap(),
        "the User's own
",
        "the wave after the change is judged under create, which changes no file"
    );
}

/// How long the generating model writes text after it hands its read
/// over, and how long that read takes to answer.
pub(super) const WRITING: std::time::Duration = std::time::Duration::from_millis(500);
pub(super) const READING: std::time::Duration = std::time::Duration::from_millis(500);

/// A read-only tool that takes `READING` to answer.
pub(super) struct SlowRead;

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
pub(super) struct GeneratingModel {
    pub(super) waves: Vec<Vec<ToolCall>>,
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

pub(super) fn one_read_while_generating(
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
        monotonic_us: &mut || 0,
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
