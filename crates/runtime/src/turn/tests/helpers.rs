// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::super::*;
use crate::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use kernel::ledger::chain_hash;
use kernel::{EventDraft, EventRef, GENESIS_PREV, RunId};

/// Minimal in-memory ledger for turn tests (the citysim MemLedger is
/// the real second adapter; this one keeps the crate's tests local).
pub(super) struct TestLedger {
    pub(super) lines: Vec<Vec<u8>>,
    next: kernel::Seq,
    prev: B3Hash,
}

impl TestLedger {
    pub(super) fn new() -> Self {
        TestLedger {
            lines: Vec::new(),
            next: kernel::Seq::FIRST,
            prev: GENESIS_PREV,
        }
    }

    pub(super) fn kinds(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_slice(line).unwrap();
                value["kind"].as_str().unwrap().to_owned()
            })
            .collect()
    }
}

impl Ledger for TestLedger {
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

pub(super) struct OneShotModel {
    pub(super) calls: Vec<ToolCall>,
}

impl Model for OneShotModel {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "thinking".to_owned(),
            }])
            .unwrap(),
            std::mem::take(&mut self.calls),
        ))
    }
}

pub(super) fn prefix() -> FrozenPrefix {
    FrozenPrefix::assemble(
        FrozenSegment::new(SegmentSlot::City, b"c".to_vec()),
        FrozenSegment::new(SegmentSlot::Building, b"b".to_vec()),
        FrozenSegment::new(SegmentSlot::Resident, b"r".to_vec()),
        FrozenSegment::new(SegmentSlot::Run, b"j".to_vec()),
    )
    .unwrap()
}

/// A conversation that outlives the turn borrowing it, for tests that
/// assemble from nothing.
pub(super) fn blank_conversation() -> &'static Conversation {
    Box::leak(Box::new(Conversation::new()))
}

/// A clock stopped at `at`: every reading a turn takes is the turn's own
/// stamp, so a test that compares ledgers compares order and payloads.
pub(super) fn stopped(at: u64) -> impl FnMut() -> Result<TimeMs, AxError> {
    move || Ok(TimeMs::new(at))
}

/// A turn opened at stamp `AT` on a clock stopped there, on lines of
/// its own that nothing reads afterwards: what reaches the ledger is what
/// the turn's own barriers carried. The clock captures nothing, so
/// leaking its box allocates nothing, and the turn can outlive the test's
/// own frames.
pub(super) fn opened<const AT: u64>() -> Turn<'static, Assembling> {
    opened_on::<AT>(Box::leak(Box::new(lines())))
}

/// A turn opened at stamp `AT` on `lines`, which the test keeps so it can
/// pay the barrier the run would pay after the turn.
pub(super) fn opened_on<const AT: u64>(lines: &mut HeldLines) -> Turn<'_, Assembling> {
    let now: &'static mut dyn FnMut() -> Result<TimeMs, AxError> =
        Box::leak(Box::new(|| -> Result<TimeMs, AxError> {
            Ok(TimeMs::new(AT))
        }));
    Turn::begin(lines, TimeMs::new(AT), now)
}

/// The lines of the run every turn test writes for.
pub(super) fn lines() -> HeldLines {
    HeldLines::open(run_id(), "resident@sim.1".into())
}

pub(super) fn run_id() -> RunId {
    RunId::parse("0198f6a2-7c4a-7bbb-9d1e-00000000000a").unwrap()
}

pub(super) fn shape() -> CallShape {
    CallShape {
        model: "script".to_owned(),
        max_tokens: kernel::Ceiling::new(512),
        effort: None,
        context_tokens: 0,
    }
}

/// Runs a wave that asks for nothing and closes the turn: the barrier
/// that carries `model_returned` to the ledger.
pub(super) fn closed(turn: Turn<'_, ToolWave>, ledger: &mut dyn Ledger) -> TurnReport {
    let recording = advance(
        turn.execute_concurrent(
            Interrupt::None,
            ledger,
            &mut |_: &ToolCall, _: TimeMs| -> Result<kernel::ToolOutcome, AxError> {
                Ok(kernel::ToolOutcome {
                    result: Payload::empty(),
                    attachments: Vec::new(),
                })
            },
            &mut |_| Interrupt::None,
        )
        .unwrap(),
    );
    advance(recording.record(Interrupt::None, ledger).unwrap())
}

pub(super) fn advance<N>(outcome: PhaseOutcome<N>) -> N {
    match outcome {
        PhaseOutcome::Advanced(next) => next,
        PhaseOutcome::Cancelled(_) => panic!("expected the phase to advance"),
    }
}

pub(super) fn probe_call() -> ToolCall {
    ToolCall {
        id: "call-1".to_owned(),
        name: kernel::ToolName::parse("probe").unwrap(),
        args: Payload::empty(),
    }
}
