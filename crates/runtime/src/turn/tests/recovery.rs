// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model-call recovery pipeline's segment contract: the relay order,
//! the three answers, what a skip hands on, and the one production
//! repair driven through both doors.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::cell::RefCell;

use super::super::ledger::Journal;
use super::super::recovery::{ModelCall, Segment, SegmentOutcome, recover};
use super::super::*;
use super::helpers::*;

/// A scripted segment: it records the order it was consulted in and the
/// failure it was offered, then answers what the scenario says.
struct Scripted<'t> {
    name: &'static str,
    trace: &'t RefCell<Vec<(&'static str, AxCode)>>,
    answer: Option<SegmentOutcome>,
}

impl Segment for Scripted<'_> {
    fn attempt(&mut self, failure: &AxError, _call: &mut ModelCall<'_>) -> SegmentOutcome {
        self.trace.borrow_mut().push((self.name, *failure.code()));
        self.answer
            .take()
            .unwrap_or_else(|| SegmentOutcome::Skipped(failure.clone()))
    }
}

fn wire_mismatch() -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "translate wire",
        "tool_calls[0].arguments: cut",
    )
    .with_recovery("check that the endpoint's dialect matches the provider")
}

fn provider_wobble() -> AxError {
    AxError::failure(
        AxCode::Provider,
        "call provider",
        "https://example answered 503",
    )
    .retriable()
    .with_recovery("the watchdog decides retry or failover; admission widens the interval")
}

fn request() -> ModelRequest {
    ModelRequest {
        policy: BuildingPolicy::default(),
        segments: prefix().segment_hashes(),
        chat: ChatRequest::empty("script", kernel::Ceiling::new(512).unwrap()),
    }
}

fn model_return() -> ModelReturn {
    ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::Text {
            text: "recovered".to_owned(),
        }])
        .unwrap(),
        Vec::new(),
    )
}

#[test]
fn the_relay_asks_segments_in_order_and_stops_at_the_first_recovery() {
    let trace = RefCell::new(Vec::new());
    let mut one = Scripted {
        name: "one",
        trace: &trace,
        answer: Some(SegmentOutcome::Skipped(wire_mismatch())),
    };
    let mut two = Scripted {
        name: "two",
        trace: &trace,
        answer: Some(SegmentOutcome::Recovered(model_return())),
    };
    let mut three = Scripted {
        name: "three",
        trace: &trace,
        answer: Some(SegmentOutcome::Failed(provider_wobble())),
    };
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel { calls: Vec::new() };
    let request = request();
    let mut journal = Journal::open(run_id(), "sim".into(), TimeMs::new(1));
    let mut call = ModelCall::open(&mut journal, &mut ledger, &mut model, &request);
    let outcome = recover(
        &mut [&mut one, &mut two, &mut three],
        &mut call,
        wire_mismatch(),
    );
    match outcome {
        SegmentOutcome::Recovered(_) => {}
        other => panic!("the second segment recovered, got {other:?}"),
    }
    assert_eq!(
        trace.into_inner(),
        vec![("one", AxCode::WireMismatch), ("two", AxCode::WireMismatch)],
        "the relay asks in roster order and stops at the first recovery"
    );
}

#[test]
fn when_every_segment_skips_the_original_failure_surfaces_unchanged() {
    let original = provider_wobble();
    let trace = RefCell::new(Vec::new());
    let mut one = Scripted {
        name: "one",
        trace: &trace,
        answer: Some(SegmentOutcome::Skipped(original.clone())),
    };
    let mut two = Scripted {
        name: "two",
        trace: &trace,
        answer: Some(SegmentOutcome::Skipped(original.clone())),
    };
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel { calls: Vec::new() };
    let request = request();
    let mut journal = Journal::open(run_id(), "sim".into(), TimeMs::new(1));
    let mut call = ModelCall::open(&mut journal, &mut ledger, &mut model, &request);
    let outcome = recover(&mut [&mut one, &mut two], &mut call, original.clone());
    match outcome {
        SegmentOutcome::Skipped(carried) => assert_eq!(
            carried, original,
            "a skip hands the failure on field for field: the retriable flag and \
             the recovery sentence the watchdog layer reads are still there"
        ),
        other => panic!("every segment skipped, got {other:?}"),
    }
    assert_eq!(
        trace.into_inner(),
        vec![("one", AxCode::Provider), ("two", AxCode::Provider)],
        "each skip saw the same original failure"
    );
}

#[test]
fn a_segment_that_cannot_repair_ends_the_relay_with_its_own_code() {
    let trace = RefCell::new(Vec::new());
    let mut one = Scripted {
        name: "one",
        trace: &trace,
        answer: Some(SegmentOutcome::Skipped(wire_mismatch())),
    };
    let blocking_failure =
        AxError::failure(AxCode::Timeout, "resend the call", "the blocking door")
            .with_recovery("raise this endpoint's timeout, then dispatch again");
    let mut two = Scripted {
        name: "two",
        trace: &trace,
        answer: Some(SegmentOutcome::Failed(blocking_failure)),
    };
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel { calls: Vec::new() };
    let request = request();
    let mut journal = Journal::open(run_id(), "sim".into(), TimeMs::new(1));
    let mut call = ModelCall::open(&mut journal, &mut ledger, &mut model, &request);
    let outcome = recover(&mut [&mut one, &mut two], &mut call, wire_mismatch());
    match outcome {
        SegmentOutcome::Failed(err) => assert_eq!(
            err.code(),
            &AxCode::Timeout,
            "the final code is the failing repair's own"
        ),
        other => panic!("the second segment failed, got {other:?}"),
    }
    assert_eq!(
        trace.into_inner(),
        vec![("one", AxCode::WireMismatch), ("two", AxCode::WireMismatch)],
        "the skip handed the original error to the repair that followed it"
    );
}

/// The one production segment: a reply the streaming door's parser
/// refused is asked for again through the blocking door, and every
/// attempt is on the ledger before it is made.
#[test]
fn a_wire_mismatch_on_the_streaming_door_is_repaired_through_the_blocking_door() {
    struct TwoDoors {
        streamed: u32,
        blocked: u32,
    }
    impl Model for TwoDoors {
        fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.blocked = self.blocked.saturating_add(1);
            Ok(model_return())
        }
        fn call_streaming(
            &mut self,
            _req: &ModelRequest,
            _onto: kernel::Increments<'_>,
        ) -> Result<ModelReturn, AxError> {
            self.streamed = self.streamed.saturating_add(1);
            Err(wire_mismatch())
        }
    }
    let mut ledger = TestLedger::new();
    let mut model = TwoDoors {
        streamed: 0,
        blocked: 0,
    };
    let mut conversation = Conversation::new();
    conversation.push_task_lines(
        "probe the city",
        "one probe",
        crate::conversation::Opening::FromJob,
    );
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &conversation,
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let mut arrived = 0u32;
    let mut sink = |_inc: &kernel::Increment| {
        arrived = arrived.saturating_add(1);
    };
    let turn = advance(
        turn.call(
            Interrupt::None,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            Some(&mut sink),
        )
        .unwrap(),
    );
    drop(turn);
    assert_eq!(
        ledger.kinds(),
        vec![
            "prompt_assembled",
            "model_called",
            "model_called",
            "model_returned"
        ],
        "the repaired resend is a recorded attempt, not a silent repeat"
    );
    assert_eq!(model.streamed, 1);
    assert_eq!(
        model.blocked, 1,
        "the repair asked once through the blocking door"
    );
}

/// A failure no segment repairs reaches the watchdog layer exactly as it
/// arrived: same code, still retriable, same recovery sentence, and no
/// second attempt anywhere behind it.
#[test]
fn a_retriable_failure_passes_through_to_the_watchdog_untouched() {
    struct Refusing {
        blocked: u32,
    }
    impl Model for Refusing {
        fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.blocked = self.blocked.saturating_add(1);
            panic!("a skip must not resend");
        }
        fn call_streaming(
            &mut self,
            _req: &ModelRequest,
            _onto: kernel::Increments<'_>,
        ) -> Result<ModelReturn, AxError> {
            Err(provider_wobble())
        }
    }
    let mut ledger = TestLedger::new();
    let mut model = Refusing { blocked: 0 };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &Conversation::new(),
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let mut sink = |_inc: &kernel::Increment| {};
    let err = turn
        .call(
            Interrupt::None,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            Some(&mut sink),
        )
        .unwrap_err();
    assert_eq!(
        err,
        provider_wobble(),
        "the original error, field for field"
    );
    assert_eq!(
        err.retry(),
        kernel::Retry::Yes,
        "the watchdog classifies it from this envelope"
    );
    assert_eq!(
        model.blocked, 0,
        "no segment acted on a failure it does not repair"
    );
    assert_eq!(
        ledger.kinds(),
        vec!["prompt_assembled", "model_called"],
        "the one attempt is the one on the ledger"
    );
}
