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
use crate::prefix::{FrozenSegment, SegmentSlot};
use kernel::{GENESIS_PREV, chain_hash};

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
            kernel::message_payload(&[ContentBlock::Text {
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

pub(super) fn run_id() -> RunId {
    RunId::parse("0198f6a2-7c4a-7bbb-9d1e-00000000000a").unwrap()
}

pub(super) fn shape() -> CallShape {
    CallShape {
        model: "script".to_owned(),
        max_tokens: 512,
        effort: None,
        context_tokens: 0,
    }
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
