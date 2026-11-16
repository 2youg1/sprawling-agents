// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use base64::Engine as _;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Answer {
    blocks: Vec<Block>,
}

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
