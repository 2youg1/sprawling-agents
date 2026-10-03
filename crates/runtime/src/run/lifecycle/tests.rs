// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What opening a run costs before the model is asked anything.

use kernel::ledger::chain_hash;
use kernel::{
    Address, B3Hash, Ceiling, ContentBlock, EventDraft, EventKind, EventRef, GENESIS_PREV, Locator,
    ModelRequest, ModelReturn, Seq, ToolCall, ToolOutcome,
};

use super::*;
use crate::RunPlan;
use crate::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use crate::run::Opening;
use crate::turn::CallShape;
use kernel::{Retries, RunId, TimeMs};

/// A ledger that records nothing but the kinds it was given and how
/// often it was asked: each call into it is one disk barrier.
struct CountingLedger {
    appended: Vec<EventKind>,
    barriers: usize,
    next: Seq,
    prev: B3Hash,
}

impl CountingLedger {
    fn new() -> CountingLedger {
        CountingLedger {
            appended: Vec::new(),
            barriers: 0,
            next: Seq::FIRST,
            prev: GENESIS_PREV,
        }
    }

    fn keep(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let kind = draft.kind;
        let record = kernel::EventRecord::from_draft(draft, self.next, self.prev);
        let line = record.canonical_line()?;
        self.prev = chain_hash(&line);
        self.next = self.next.next()?;
        self.appended.push(kind);
        Ok(record.to_ref())
    }
}

impl Ledger for CountingLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        self.barriers = self.barriers.saturating_add(1);
        self.keep(draft)
    }

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        self.barriers = self.barriers.saturating_add(1);
        drafts.into_iter().map(|draft| self.keep(draft)).collect()
    }
}

/// A model that answers once with text and asks for no tool.
struct Answering;

impl Model for Answering {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "done".to_owned(),
            }])?,
            Vec::new(),
        ))
    }
}

fn plan() -> RunPlan {
    let addr = Address::parse("lab/room1").expect("a canonical address");
    let job = Locator::parse(&format!("file:{}/JOB.md@{}", addr.as_str(), "a".repeat(40)))
        .expect("a canonical locator");
    RunPlan {
        run: RunId::from_bytes([7; 16]),
        who: "lab/room1".to_owned(),
        addr,
        task: "close the loop".to_owned(),
        goal: "one turn, then stop".to_owned(),
        opening: Opening::FromJob,
        job,
        parent: None,
        predecessor: None,
        dispatched_by: kernel::event::Who::Person,
        run_policy: kernel::RunPolicy::of(kernel::Mode::Work),
        naming: None,
        inherited: Vec::new(),
        shape: CallShape {
            model: "script".to_owned(),
            max_tokens: Ceiling::new(4096),
            effort: None,
            context_tokens: 0,
        },
        second_threshold: None,
        context: crate::ContextReading::default(),
        prefix: FrozenPrefix::assemble(
            FrozenSegment::new(SegmentSlot::City, b"city".to_vec()),
            FrozenSegment::new(SegmentSlot::Building, b"building".to_vec()),
            FrozenSegment::new(SegmentSlot::Resident, b"resident".to_vec()),
            FrozenSegment::new(SegmentSlot::Run, b"run".to_vec()),
        )
        .expect("four segments in slot order"),
        policy: kernel::BuildingPolicy::default(),
        tools: Vec::new(),
        skills: Vec::new(),
        retries: Retries::UntilHalted,
    }
}

/// The effort a run's requests froze is the one its first line records
/// (`crates/kernel/Spec.lean` §8-85), so a page can say it before the run
/// commits anything.
#[test]
fn the_charter_carries_the_effort_the_request_froze() {
    let mut frozen = plan();
    frozen.shape.effort = Some(kernel::Effort::High);
    assert_eq!(frozen.charter().effort, Some(kernel::Effort::High));
    assert_eq!(
        plan().charter().effort,
        None,
        "an unstated effort stays unstated"
    );
}

/// What a dispatch costs before the model is asked anything, counted
/// rather than timed: two ledger lines and two clock samples, and the
/// two lines are a fact about the city and a fact about the run.
///
/// The clock is injected, so the reading is the same on every machine
/// and a third sample appearing here is a defect rather than weather.
#[test]
fn a_dispatch_writes_two_lines_and_samples_the_clock_twice() {
    let mut ledger = CountingLedger::new();
    let mut samples = 0u32;
    let mut now = || {
        samples = samples.saturating_add(1);
        Ok(TimeMs::new(u64::from(samples)))
    };
    let mut interrupt = |_: SafePoint| crate::turn::Interrupt::None;
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
        wait: &mut |_: TimeMs| crate::NextCall::Allowed,
        deltas: None,
    };
    {
        let run = Run::dispatch(plan(), &mut ledger, &mut hooks).expect("the dispatch pair lands");
        assert_eq!(run.turns_taken(), 0, "a dispatch takes no turn");
    }

    eprintln!(
        "dispatch_prelude: {} ledger appends and {samples} clock samples before the \
         first model call",
        ledger.appended.len()
    );
    assert_eq!(
        ledger.appended,
        vec![EventKind::CheckpointCommitted, EventKind::RunStarted],
        "the job pin lands first, then the run exists"
    );
    assert_eq!(samples, 2, "one stamp per fact, and no third sample");
}

/// `closed_turn_barriers` through the run: the run's own
/// `prompt_shape_compared` waits among the turn's lines, so a run's first
/// turn that calls no tool pays one barrier for its model call, which
/// carries `prompt_assembled` too, and one to close.
#[test]
fn tf1_run_turn_barriers() {
    let mut ledger = CountingLedger::new();
    let mut now = || Ok(TimeMs::new(1));
    let mut interrupt = |_: SafePoint| crate::turn::Interrupt::None;
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
        wait: &mut |_: TimeMs| crate::NextCall::Allowed,
        deltas: None,
    };
    let mut run = Run::dispatch(plan(), &mut ledger, &mut hooks).unwrap();
    let opened = ledger.barriers;
    run.advance(&mut ledger, &mut Answering, &mut hooks)
        .unwrap();
    assert_eq!(
        (ledger.barriers - opened, &ledger.appended[2..]),
        (
            2,
            &[
                EventKind::PromptAssembled,
                EventKind::PromptShapeCompared,
                EventKind::ModelCalled,
                EventKind::ModelReturned,
            ][..]
        )
    );
}
