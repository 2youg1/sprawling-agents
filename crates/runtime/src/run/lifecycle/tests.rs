// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What opening a run costs before the model is asked anything.

use kernel::ledger::chain_hash;
use kernel::{Address, B3Hash, Ceiling, EventRef, GENESIS_PREV, Locator, Seq, ToolOutcome};

use super::*;
use crate::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use crate::run::Opening;
use crate::turn::CallShape;
use crate::{Retries, RunPlan};

/// A ledger that records nothing but how often it was asked.
struct CountingLedger {
    appended: Vec<EventKind>,
    next: Seq,
    prev: B3Hash,
}

impl Ledger for CountingLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let kind = draft.kind;
        let record = kernel::EventRecord::from_draft(draft, self.next, self.prev);
        let line = record.canonical_line()?;
        self.prev = chain_hash(&line);
        self.next = self.next.next()?;
        self.appended.push(kind);
        Ok(record.to_ref())
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
        shape: CallShape {
            model: "script".to_owned(),
            max_tokens: Ceiling::new(4096),
            effort: None,
            context_tokens: 0,
        },
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

/// What a dispatch costs before the model is asked anything, counted
/// rather than timed: two ledger lines and two clock samples, and the
/// two lines are a fact about the city and a fact about the run.
///
/// The clock is injected, so the reading is the same on every machine
/// and a third sample appearing here is a defect rather than weather.
#[test]
fn a_dispatch_writes_two_lines_and_samples_the_clock_twice() {
    let mut ledger = CountingLedger {
        appended: Vec::new(),
        next: Seq::FIRST,
        prev: GENESIS_PREV,
    };
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
        interrupt: &mut interrupt,
        fence: None,
        invoke: &mut invoke,
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
