// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a `tools/call` that was carried out answers.
//!
//! MCP gives a tool's answer one shape, `CallToolResult`: an array of
//! content blocks. The facts of an answer travel as one text block
//! holding their JSON on one line, a picture travels as an image block,
//! so a model that reads images sees the picture rather than a page of
//! base64 (`crates/desktop/Spec.lean` D2), and a recording's sound
//! travels as an audio block the city stores (`crates/desktop/Spec.lean` D13). The
//! base64 is written here and nowhere else.

use base64::Engine as _;
use serde_json::{Value, json};

/// One answer, as the blocks it is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Answer {
    blocks: Vec<Block>,
}

/// One content block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(any(windows, test)),
    expect(
        dead_code,
        reason = "the non-Windows arm refuses every call, so it builds no answer"
    )
)]
enum Block {
    Text(String),
    Image { bytes: Vec<u8>, mime: &'static str },
    Sound { bytes: Vec<u8>, mime: &'static str },
}

impl Answer {
    /// The facts of an answer, as one text block of single-line JSON.
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "the non-Windows arm refuses every call, so it builds no answer"
        )
    )]
    pub(crate) fn facts(facts: &Value) -> Answer {
        Answer {
            blocks: vec![Block::Text(facts.to_string())],
        }
    }

    /// An outline of a window, then the facts about it: the outline
    /// comes first because it is what was asked for, and it travels as
    /// the text it already is rather than as a JSON string of it.
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "the non-Windows arm refuses every call, so it builds no answer"
        )
    )]
    pub(crate) fn outline(outline: String, facts: &Value) -> Answer {
        Answer {
            blocks: vec![Block::Text(outline), Block::Text(facts.to_string())],
        }
    }

    /// A picture, then the facts about it: the image block comes first
    /// because it is what was asked for, and the text beside it says
    /// what the picture is.
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "the non-Windows arm refuses every call, so it builds no answer"
        )
    )]
    pub(crate) fn picture(bytes: Vec<u8>, mime: &'static str, facts: &Value) -> Answer {
        Answer {
            blocks: vec![Block::Image { bytes, mime }, Block::Text(facts.to_string())],
        }
    }

    /// A recording's sound, then the facts about the recording: the
    /// same order a picture takes, so the city reads both answers one
    /// way (`crates/desktop/Spec.lean` D13).
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "the non-Windows arm refuses every call, so it builds no answer"
        )
    )]
    pub(crate) fn sound(bytes: Vec<u8>, mime: &'static str, facts: &Value) -> Answer {
        Answer {
            blocks: vec![Block::Sound { bytes, mime }, Block::Text(facts.to_string())],
        }
    }

    /// The `result` of the JSON-RPC answer.
    pub(crate) fn as_result(&self) -> Value {
        let content: Vec<Value> = self
            .blocks
            .iter()
            .map(|block| match block {
                Block::Text(text) => json!({ "type": "text", "text": text }),
                Block::Image { bytes, mime } => json!({
                    "type": "image",
                    "data": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "mimeType": mime,
                }),
                Block::Sound { bytes, mime } => json!({
                    "type": "audio",
                    "data": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "mimeType": mime,
                }),
            })
            .collect();
        json!({ "content": content })
    }
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
    fn a_picture_travels_as_image_content_before_its_facts() {
        let answer = Answer::picture(
            b"\x89PNG".to_vec(),
            "image/png",
            &json!({ "title": "a.txt", "width": 1, "height": 1, "lossless": true }),
        );
        assert_eq!(
            answer.as_result(),
            json!({ "content": [
                { "type": "image", "data": "iVBORw==", "mimeType": "image/png" },
                { "type": "text", "text": "{\"height\":1,\"lossless\":true,\"title\":\"a.txt\",\"width\":1}" },
            ]})
        );
    }

    /// A recording's sound reaches the city as MCP audio content, in
    /// front of the facts, the same shape a screenshot has
    /// (`crates/desktop/Spec.lean` D13).
    #[test]
    fn a_recordings_sound_travels_as_audio_content_before_its_facts() {
        let answer = Answer::sound(
            b"RIFF".to_vec(),
            "audio/wav",
            &json!({ "state": "stopped", "recording": 1 }),
        );
        assert_eq!(
            answer.as_result(),
            json!({ "content": [
                { "type": "audio", "data": "UklGRg==", "mimeType": "audio/wav" },
                { "type": "text", "text": "{\"recording\":1,\"state\":\"stopped\"}" },
            ]})
        );
    }

    #[test]
    fn an_outline_travels_as_its_own_text_before_its_facts() {
        assert_eq!(
            Answer::outline("e1 window \"w\"\n".to_owned(), &json!({ "generation": 1 }))
                .as_result(),
            json!({ "content": [
                { "type": "text", "text": "e1 window \"w\"\n" },
                { "type": "text", "text": "{\"generation\":1}" },
            ]})
        );
    }

    #[test]
    fn facts_travel_as_one_line_of_text() {
        assert_eq!(
            Answer::facts(&json!({ "operation": "set" })).as_result(),
            json!({ "content": [{ "type": "text", "text": "{\"operation\":\"set\"}" }] })
        );
    }
}
