// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which statement about what one model accepts wins (shape 1 decision;
//! `crates/gateway/spec/Provider/Input.lean` §8-37).
//!
//! **A model whose vendor documents that it reads pictures was
//! registered as reading only text.** The registration took its input
//! from the pinned catalogue alone, which holds one row that reads
//! pictures, while every model row of the preset table already states
//! its input beside the page it was read from. The endpoint then
//! refused every picture sent to that model, and the ocr tool with it.
//!
//! The window and the output ceiling already climb from the catalogue to
//! the preset table to a default; this is the same ladder for the third
//! fact a registration carries, decided here and nowhere else.

use crate::market::InputKinds;

/// What one model may be sent, from the first rung that states it.
///
/// `stated` is what the person said in `SelectModel.input`, and outranks
/// everything in either direction (gateway D16). `pinned` is the pinned
/// catalogue's row for this exact id; the preset table is asked with the
/// base URL and the id, by the same prefix rule that answers for its
/// window and ceiling. Nobody stating anything is [`InputKinds::Text`]:
/// guessing too little costs a refusal naming the fix, guessing too much
/// costs the provider's 400.
#[must_use]
pub fn accepted_input(
    stated: Option<InputKinds>,
    pinned: Option<InputKinds>,
    base_url: &str,
    id: &str,
) -> InputKinds {
    first_stated(stated, pinned, super::preset::input_for(base_url, id))
}

/// The ladder itself, with the preset table's answer already read:
/// `Gateway.Provider.Input.accepted_input` over the same three rungs.
/// The preset lookup stays in the caller, so a check can walk every
/// combination the model admits without a host to look up.
fn first_stated(
    stated: Option<InputKinds>,
    pinned: Option<InputKinds>,
    preset: Option<InputKinds>,
) -> InputKinds {
    stated.or(pinned).or(preset).unwrap_or(InputKinds::Text)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// One table, every rung: the person outranks both tables in both
    /// directions, the catalogue outranks the preset table in both
    /// directions, the preset table answers where the catalogue is silent
    /// (at the vendor's host and at a relay forwarding the vendor's id),
    /// and a server on this machine or an id nobody documents reads text.
    #[test]
    fn the_first_rung_that_states_a_fact_answers() {
        let vendor = "https://api.anthropic.com/v1";
        let relay = "https://relay.example.test/v1";
        let local = "http://127.0.0.1:8000/v1";
        let (text, image) = (Some(InputKinds::Text), Some(InputKinds::TextImage));
        let asked = [
            (image, text, local, "local"),
            (text, None, vendor, "claude-sonnet-4-5"),
            (None, text, vendor, "claude-sonnet-4-5"),
            (None, image, local, "llava"),
            (None, None, vendor, "claude-sonnet-4-5"),
            (None, None, relay, "claude-sonnet-4-5"),
            (None, None, local, "claude-sonnet-4-5"),
            (
                None,
                None,
                "https://api.openai.com/v1",
                "some-unreleased-model",
            ),
        ];
        let answered: Vec<InputKinds> = asked
            .iter()
            .map(|(stated, pinned, base_url, id)| accepted_input(*stated, *pinned, base_url, id))
            .collect();
        assert_eq!(
            answered,
            vec![
                InputKinds::TextImage,
                InputKinds::Text,
                InputKinds::Text,
                InputKinds::TextImage,
                InputKinds::TextImage,
                InputKinds::TextImage,
                InputKinds::Text,
                InputKinds::Text,
            ]
        );
    }

    /// `every_answer_is_a_stated_fact_or_text`, checked on all 27
    /// combinations of the three rungs, each silent or stating one of the
    /// two kinds: the answer is the first rung that spoke, or `Text`
    /// when none did.
    #[test]
    fn every_answer_is_a_stated_fact_or_text_on_every_combination() {
        let said = [None, Some(InputKinds::Text), Some(InputKinds::TextImage)];
        let mut walked = 0_u32;
        for stated in said {
            for pinned in said {
                for preset in said {
                    let answer = first_stated(stated, pinned, preset);
                    let holds = stated == Some(answer)
                        || (stated.is_none() && pinned == Some(answer))
                        || (stated.is_none() && pinned.is_none() && preset == Some(answer))
                        || (stated.is_none()
                            && pinned.is_none()
                            && preset.is_none()
                            && answer == InputKinds::Text);
                    assert!(
                        holds,
                        "{stated:?} {pinned:?} {preset:?} answered {answer:?}"
                    );
                    walked += 1;
                }
            }
        }
        assert_eq!(walked, 27);
    }
}
