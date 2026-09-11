// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The context reminder, driven through the run loop: what the provider
//! reports as `input_tokens` is what the gauge reads, each threshold
//! sounds once, and the line lands where a steer lands.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{
    Address, AxError, B3Hash, BuildingPolicy, ContentBlock, EventDraft, EventRef, GENESIS_PREV,
    Ledger, Locator, Model, ModelRequest, ModelReturn, ModelUsage, Payload, RunId, Seq, TimeMs,
    Tokens, ToolCall, ToolName, ToolOutcome, message_payload,
};
use runtime::handoff::Handoff;
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use runtime::run::{RunHooks, RunPlan, drive};
use runtime::turn::CallShape;
use runtime::{Interrupt, SafePoint};

struct SinkLedger {
    seq: u64,
}

impl Ledger for SinkLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let record = kernel::EventRecord::from_draft(draft, Seq::new(self.seq), GENESIS_PREV);
        self.seq = self.seq.saturating_add(1);
        Ok(record.to_ref())
    }
}

/// A model that reports one `input_tokens` figure per turn and makes a
/// tool call on every turn but the last, so the run takes as many turns
/// as there are figures.
struct MeteredModel {
    reported: Vec<u64>,
    seen: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Model for MeteredModel {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.seen
            .borrow_mut()
            .push(format!("{:?}", req.chat.messages));
        let input = self.reported.remove(0);
        let calls = if self.reported.is_empty() {
            Vec::new()
        } else {
            vec![ToolCall {
                id: format!("t-{input}"),
                name: ToolName::parse("status").unwrap(),
                args: Payload::empty(),
            }]
        };
        let mut returned = ModelReturn::bare(
            message_payload(&[ContentBlock::Text {
                text: "working".to_owned(),
            }])
            .unwrap(),
            calls,
        );
        returned.usage = Some(ModelUsage {
            input_tokens: Tokens::new(input),
            ..ModelUsage::default()
        });
        Ok(returned)
    }
}

fn plan(window: u64) -> RunPlan {
    let addr = Address::parse("lab/room1").unwrap();
    RunPlan {
        run: RunId::from_bytes([9; 16]),
        who: "resident".to_owned(),
        addr: addr.clone(),
        task: "fill the window".to_owned(),
        goal: "until told".to_owned(),
        opening: runtime::Opening::FromJob,
        job: Locator::parse(&format!("file:{}/JOB.md@{}", addr.as_str(), "a".repeat(40))).unwrap(),
        parent: None,
        predecessor: None,
        shape: CallShape {
            model: "metered".to_owned(),
            max_tokens: kernel::Ceiling::new(4096),
            effort: None,
            context_tokens: window,
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
        vec![Locator::parse(&format!("cas:b3-{}", B3Hash::digest(b"x"))).unwrap()],
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap()
}

/// Every window the model saw, given one `input_tokens` figure per turn
/// against a window of one thousand.
fn windows_for(reported: Vec<u64>) -> Vec<String> {
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut model = MeteredModel {
        reported,
        seen: std::rc::Rc::clone(&seen),
    };
    let mut ledger = SinkLedger { seq: 1 };
    let mut tick = 0u64;
    let mut now = move || {
        tick = tick.saturating_add(1);
        Ok(TimeMs::new(tick))
    };
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
    drive(plan(1_000), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
    seen.borrow().clone()
}

#[test]
fn the_quarter_mark_reports_usage_once_and_the_handover_mark_says_what_is_left() {
    let windows = windows_for(vec![100, 300, 400, 700, 800]);
    assert_eq!(windows.len(), 5);
    assert!(
        !windows[1].contains("[context]"),
        "10% says nothing: {}",
        windows[1]
    );
    assert!(
        windows[2].contains("30% of the window used (300 of 1000 input tokens)"),
        "past a quarter, the usage is reported as the provider counted it: {}",
        windows[2]
    );
    assert!(
        windows[4].contains("70% of the window used")
            && windows[4].contains("still enough to write a handoff and `succeed`"),
        "past two thirds, the run is told the budget left is still enough to hand over: {}",
        windows[4]
    );
    let last = &windows[4];
    assert_eq!(
        last.matches("of the window used").count(),
        2,
        "each threshold sounds exactly once per run: {last}"
    );
}

#[test]
fn a_jump_past_both_thresholds_sounds_the_higher_one_only() {
    let windows = windows_for(vec![700, 900]);
    let last = &windows[1];
    assert!(last.contains("still enough to write a handoff"), "{last}");
    assert_eq!(last.matches("of the window used").count(), 1, "{last}");
}

#[test]
fn a_model_with_no_stated_window_is_never_reminded() {
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut model = MeteredModel {
        reported: vec![900, 950],
        seen: std::rc::Rc::clone(&seen),
    };
    let mut ledger = SinkLedger { seq: 1 };
    let mut now = || Ok(TimeMs::new(1));
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
    drive(plan(0), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
    assert!(
        !seen.borrow()[1].contains("[context]"),
        "no denominator, no percentage: {}",
        seen.borrow()[1]
    );
}
