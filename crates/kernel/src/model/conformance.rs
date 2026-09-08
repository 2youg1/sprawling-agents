// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One assertion suite for every model implementation (V3).

use super::image::{ImageRef, ImageType};
use super::wire::{ChatMessage, ContentBlock, Role};
use super::{Model, ModelRequest};
use crate::locator::Locator;

/// The universal model contract is thin on purpose: two consecutive
/// calls must both *return* — no panic, and no poisoned state after an
/// Err. Result shapes are already the types' business; determinism is
/// not asserted (real providers are not deterministic — scripted
/// adapters prove theirs in citysim).
#[allow(
    clippy::panic,
    reason = "conformance suites assert by panicking; they are dev-only by feature"
)]
#[cfg(feature = "conformance")]
pub fn assert_model_conformance<M: Model>(model: &mut M, benign: &ModelRequest) {
    for round in 0..2u8 {
        match model.call(benign) {
            Ok(ret) => assert!(
                ret.calls.len() != usize::MAX,
                "round {round}: adapter returned a wave"
            ),
            Err(err) => assert!(
                !err.code().as_str().is_empty(),
                "round {round}: adapter returned a typed error"
            ),
        }
    }
    assert_seeing_a_picture_answers(model, benign);
}

/// A third round with one picture in the conversation.
///
/// The contract is the same one the two benign rounds state — an answer
/// or a typed error, never a panic — and it is asserted separately
/// because an adapter that has never met an `Image` block is exactly the
/// adapter that would index past the end of a match arm.
#[allow(
    clippy::panic,
    reason = "conformance suites assert by panicking; they are dev-only by feature"
)]
#[cfg(feature = "conformance")]
fn assert_seeing_a_picture_answers<M: Model>(model: &mut M, benign: &ModelRequest) {
    let Ok(locator) = Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))) else {
        panic!("the conformance picture's locator is spelled by this file");
    };
    let mut seeing = benign.clone();
    seeing.chat.messages.push(ChatMessage {
        role: Role::User,
        content: vec![ContentBlock::Image(ImageRef {
            locator,
            media_type: ImageType::Png,
            width: 16,
            height: 16,
        })],
    });
    match model.call(&seeing) {
        Ok(ret) => assert!(
            ret.calls.len() != usize::MAX,
            "seeing a picture: adapter returned a wave"
        ),
        Err(err) => assert!(
            !err.code().as_str().is_empty(),
            "seeing a picture: adapter returned a typed error"
        ),
    }
}
