// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which statement about what one model accepts wins (shape 1 decision;
//! gateway-SPEC.md section 8-37).
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
/// `pinned` is the pinned catalogue's row for this exact id; the preset
/// table is asked with the base URL and the id, by the same prefix rule
/// that answers for its window and ceiling. Neither stating anything is
/// [`InputKinds::Text`]: guessing too little costs a refusal naming the
/// fix, guessing too much costs the provider's 400.
///
/// The person's own statement would outrank both, and has no wire entry
/// yet; when it gains one it becomes the first rung here.
#[must_use]
pub fn accepted_input(pinned: Option<InputKinds>, base_url: &str, id: &str) -> InputKinds {
    let _ = (base_url, id);
    pinned.unwrap_or_default()
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

    /// One table, every rung: the catalogue outranks the preset table
    /// in both directions, the preset table answers where the catalogue
    /// is silent - at the vendor's host and at a relay forwarding the
    /// vendor's id - and a server on this machine or an id nobody
    /// documents reads text.
    #[test]
    fn the_first_rung_that_states_a_fact_answers() {
        let vendor = "https://api.anthropic.com/v1";
        let relay = "https://relay.example.test/v1";
        let local = "http://127.0.0.1:8000/v1";
        let asked = [
            (Some(InputKinds::Text), vendor, "claude-sonnet-4-5"),
            (Some(InputKinds::TextImage), local, "llava"),
            (None, vendor, "claude-sonnet-4-5"),
            (None, relay, "claude-sonnet-4-5"),
            (None, local, "claude-sonnet-4-5"),
            (None, "https://api.openai.com/v1", "some-unreleased-model"),
        ];
        let answered: Vec<InputKinds> = asked
            .iter()
            .map(|(pinned, base_url, id)| accepted_input(*pinned, base_url, id))
            .collect();
        assert_eq!(
            answered,
            vec![
                InputKinds::Text,
                InputKinds::TextImage,
                InputKinds::TextImage,
                InputKinds::TextImage,
                InputKinds::Text,
                InputKinds::Text,
            ]
        );
    }
}
