// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When a reply's first content arrived: read from the turn's clock on
//! every streaming door, whether or not a page is watching, and on no
//! other door (runtime-SPEC 8-50).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::super::*;
use super::helpers::*;
use kernel::{Increment, Increments, ToolOutcome};

/// A model that streams an empty piece of reasoning, then a real one,
/// then answers.
struct Streaming;

impl Model for Streaming {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "weighed".to_owned(),
            }])
            .unwrap(),
            Vec::new(),
        ))
    }

    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        for piece in ["", "weighing it"] {
            onto(&Increment::Thought(piece.to_owned()));
        }
        self.call(req)
    }
}

/// A turn on a clock that moves ten milliseconds each time it is read.
fn ticking() -> Turn<'static, Assembling> {
    let ticks: &'static mut u64 = Box::leak(Box::new(0));
    let now: &'static mut dyn FnMut() -> Result<TimeMs, AxError> =
        Box::leak(Box::new(move || -> Result<TimeMs, AxError> {
            *ticks = ticks.saturating_add(10);
            Ok(TimeMs::new(*ticks))
        }));
    Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1), now)
}

/// The `t` of `model_called`, the `first_at` of `model_returned` and the
/// `t` of `model_returned`, after one call through `generating`.
fn readings(generating: Generating<'_, '_>) -> (Option<u64>, Option<u64>, Option<u64>) {
    let mut ledger = TestLedger::new();
    let turn = advance(
        ticking()
            .assemble(
                Interrupt::None,
                &mut ledger,
                RunPrompt::new(&prefix(), &mut PromptRecord::default()),
                blank_conversation(),
                &[],
                &shape(),
            )
            .unwrap(),
    );
    drop(advance(
        turn.call(
            Interrupt::None,
            &mut ledger,
            &mut Streaming,
            &BuildingPolicy::default(),
            generating,
        )
        .unwrap(),
    ));
    let line =
        |at: usize| -> serde_json::Value { serde_json::from_slice(&ledger.lines[at]).unwrap() };
    let (called, returned) = (line(1), line(2));
    (
        called["t"].as_u64(),
        returned["data"]["first_at"].as_u64(),
        returned["t"].as_u64(),
    )
}

#[test]
fn a_streamed_reply_records_when_its_first_content_arrived() {
    let no_tools = |_: &ToolCall, _: TimeMs| -> Result<ToolOutcome, AxError> {
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    };
    assert_eq!(
        readings(Generating::Speculating {
            deltas: None,
            tools: &no_tools,
        }),
        (Some(10), Some(20), Some(30)),
        "sent, first content, whole: three readings of one clock, the empty piece \
         not among them, and no page watching"
    );
}

#[test]
fn a_watched_stream_hands_the_page_every_piece_and_still_records_the_first() {
    let mut seen = Vec::new();
    let mut page = |piece: &Increment| seen.push(piece.clone());
    let read = readings(Generating::Watched(&mut page));
    assert_eq!(read, (Some(10), Some(20), Some(30)));
    assert_eq!(
        seen,
        vec![
            Increment::Thought(String::new()),
            Increment::Thought("weighing it".to_owned())
        ],
        "the page reads the stream as the model sent it"
    );
}

#[test]
fn the_blocking_door_records_no_first_content() {
    assert_eq!(
        readings(Generating::Unwatched),
        (Some(10), None, Some(20)),
        "a door that reports nothing before it settles has no first content to time"
    );
}
