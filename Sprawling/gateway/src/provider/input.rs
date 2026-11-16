// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::market::InputKinds;

#[must_use]
pub fn accepted_input(
    stated: Option<InputKinds>,
    pinned: Option<InputKinds>,
    base_url: &str,
    id: &str,
) -> InputKinds {
    first_stated(stated, pinned, super::preset::input_for(base_url, id))
}

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
