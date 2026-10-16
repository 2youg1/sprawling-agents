// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wait an attempt took at its endpoint's gate reaches the history
//! as a `prefix_held` line after that attempt's `model_called`, on
//! success and on failure (`crates/runtime/spec/Turn/Recovery.lean`
//! D98), and the request says which part of it a building shares
//! (`crates/runtime/spec/Prefix.lean` D97).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::num::NonZeroUsize;

use super::super::*;
use super::helpers::*;
use kernel::event::record::{PrefixHeld, PrewarmEnd, PrewarmSent};
use kernel::{ProviderFailureKind, SharedPrefix};

/// A model whose every call waited behind a pre-warm it sent, which the
/// provider refused; the call itself answers, or fails when `fails`.
struct Held {
    fails: bool,
    shared: Option<SharedPrefix>,
}

fn refused() -> AxError {
    AxError::provider(
        ProviderFailureKind::Refused { status: 429 },
        "call provider",
        "429",
    )
    .with_recovery("wait")
}

impl Model for Held {
    fn prefix_held(&self) -> Option<PrefixHeld> {
        Some(PrefixHeld {
            waited_us: 7,
            ended: PrewarmEnd::Refused,
            sent: Some(PrewarmSent::Refused { refused: refused() }),
        })
    }

    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.shared = Some(req.chat.shared);
        if self.fails {
            return Err(refused());
        }
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "done".to_owned(),
            }])
            .unwrap(),
            Vec::new(),
        ))
    }
}

/// The kinds the ledger holds after one turn's call through `model`, and
/// the `prefix_held` line's payload.
fn after_one_call(model: &mut Held) -> (Vec<String>, Option<PrefixHeld>) {
    let mut ledger = TestLedger::new();
    let mut lines = lines();
    let turn = advance(
        opened_on::<1>(&mut lines)
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
    if let Ok(wave) = turn.call(
        Interrupt::None,
        &mut ledger,
        model,
        &BuildingPolicy::default(),
        Generating::Unwatched,
    ) {
        let recording = advance(
            advance(wave)
                .execute_concurrent(
                    Interrupt::None,
                    &mut ledger,
                    &mut |_: &ToolCall, _: TimeMs| -> Result<kernel::ToolOutcome, AxError> {
                        panic!("the reply calls no tool")
                    },
                    &mut |_| Interrupt::None,
                )
                .unwrap(),
        );
        advance(recording.record(Interrupt::None, &mut ledger).unwrap());
    }
    lines.barrier(&mut ledger).unwrap();
    let held = ledger.lines.iter().find_map(|line| {
        let value: serde_json::Value = serde_json::from_slice(line).unwrap();
        (value["kind"] == "prefix_held")
            .then(|| serde_json::from_value(value["data"].clone()).unwrap())
    });
    (ledger.kinds(), held)
}

#[test]
fn a_held_attempt_is_recorded_between_its_call_and_its_return() {
    let mut answering = Held {
        fails: false,
        shared: None,
    };
    let (kinds, held) = after_one_call(&mut answering);
    assert_eq!(
        (kinds, held, answering.shared),
        (
            vec![
                "prompt_assembled".to_owned(),
                "model_called".to_owned(),
                "prefix_held".to_owned(),
                "model_returned".to_owned(),
            ],
            answering.prefix_held(),
            Some(SharedPrefix::Leading(NonZeroUsize::new(2).unwrap())),
        )
    );
}

#[test]
fn a_held_attempt_that_failed_is_recorded_too() {
    let mut failing = Held {
        fails: true,
        shared: None,
    };
    let (kinds, held) = after_one_call(&mut failing);
    assert_eq!(
        (kinds, held),
        (
            vec![
                "prompt_assembled".to_owned(),
                "model_called".to_owned(),
                "prefix_held".to_owned(),
            ],
            failing.prefix_held(),
        )
    );
}
