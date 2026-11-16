// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
    Address, AxError, B3Hash, BuildingPolicy, ContentBlock, EventDraft, EventRef, GENESIS_PREV,
    Ledger, Locator, Model, ModelRequest, ModelReturn, Payload, RunId, TimeMs, ToolCall, ToolName,
};
use runtime::handoff::Handoff;
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use runtime::run::RunPlan;
use runtime::turn::CallShape;

pub(super) struct RecordingLedger {
    pub(super) lines: Vec<Vec<u8>>,
    next: kernel::Seq,
    prev: B3Hash,
}

impl RecordingLedger {
    pub(super) fn new() -> Self {
        RecordingLedger {
            lines: Vec::new(),
            next: kernel::Seq::FIRST,
            prev: GENESIS_PREV,
        }
    }

    pub(super) fn field(&self, key: &str) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_slice(line).unwrap();
                value[key].to_string().trim_matches('"').to_owned()
            })
            .collect()
    }

    pub(super) fn kinds(&self) -> Vec<String> {
        self.field("kind")
    }

    pub(super) fn stamps(&self) -> Vec<u64> {
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
pub(super) struct ScriptedModel {
    pub(super) waves: Vec<Vec<ToolCall>>,
    /// Every user message this model was shown, in order, so a test can
    /// ask what actually reached the window rather than what was
    /// recorded about it.
    pub(super) seen: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
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
pub(super) struct FailingModel {
    pub(super) code: kernel::AxCode,
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
pub(super) struct FlakyModel {
    pub(super) failures_left: u32,
    pub(super) calls: std::rc::Rc<std::cell::RefCell<u32>>,
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

pub(super) fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("status").unwrap(),
        args: Payload::empty(),
    }
}

pub(super) fn plan() -> RunPlan {
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
        routing: None,
        run_policy: runtime::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)),
        naming: None,
        inherited: Vec::new(),
        shape: CallShape {
            model: "script".to_owned(),
            generation: kernel::FrozenGeneration {
                max_tokens: kernel::Ceiling::new(4096),
                ..kernel::FrozenGeneration::default()
            },
            effort: None,
            context_tokens: 0,
        },
        second_threshold: None,
        context_lock: None,
        succession: std::sync::Arc::new(std::sync::Mutex::new(runtime::SuccessionDesk::new())),
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

pub(super) fn handoff() -> Handoff {
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
pub(super) fn counter() -> impl FnMut() -> Result<TimeMs, AxError> {
    let mut tick: u64 = 0;
    move || {
        let now = TimeMs::new(tick);
        tick = tick.saturating_add(1);
        Ok(now)
    }
}
