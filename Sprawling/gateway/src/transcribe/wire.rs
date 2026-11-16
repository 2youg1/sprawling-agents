// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::Value;

use kernel::AxError;

use crate::mismatch::{as_str, require};

use super::recording::Recording;

pub(crate) fn transcription_path() -> &'static str {
    "audio/transcriptions"
}

const BOUNDARY_SEED: &str = "----sprawling-recording-boundary";

pub(crate) struct FormBody {
    pub(crate) content_type: String,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn form_body(model: &str, recording: &Recording) -> FormBody {
    let boundary = boundary_for(recording.bytes());
    let mut bytes = Vec::new();
    push_text_field(&mut bytes, &boundary, "model", model);
    bytes.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    bytes.extend_from_slice(
        format!(
            "content-disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
             content-type: {}\r\n\r\n",
            recording.kind().file_name(),
            recording.kind().media_type(),
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(recording.bytes());
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    FormBody {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        bytes,
    }
}

fn push_text_field(into: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    into.extend_from_slice(
        format!(
            "--{boundary}\r\ncontent-disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        )
        .as_bytes(),
    );
}

fn boundary_for(audio: &[u8]) -> String {
    let mut boundary = BOUNDARY_SEED.to_owned();
    while contains(audio, boundary.as_bytes()) {
        boundary.push('-');
    }
    boundary
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub(crate) fn transcription_of(wire: &Value) -> Result<String, AxError> {
    let text = require(wire, "transcription", "text")?;
    Ok(as_str(text, "transcription.text")?.to_owned())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::recording::AudioType;
    use super::*;
    use kernel::AxCode;

    fn recording(bytes: &[u8]) -> Recording {
        Recording::new(bytes.to_vec(), AudioType::Wav).unwrap()
    }

    #[test]
    fn the_body_carries_the_model_the_filename_and_the_bytes() {
        let body = form_body("whisper-1", &recording(b"RIFFfake"));
        let text = String::from_utf8_lossy(&body.bytes).into_owned();
        assert!(
            body.content_type
                .starts_with("multipart/form-data; boundary=")
        );
        assert!(text.contains("name=\"model\"\r\n\r\nwhisper-1\r\n"));
        assert!(text.contains("filename=\"recording.wav\""));
        assert!(text.contains("content-type: audio/wav"));
        assert!(text.contains("RIFFfake"));
        assert!(text.ends_with("--\r\n"));
    }

    #[test]
    fn the_same_recording_always_produces_the_same_bytes() {
        let first = form_body("whisper-1", &recording(b"RIFFfake"));
        let second = form_body("whisper-1", &recording(b"RIFFfake"));
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.content_type, second.content_type);
    }

    #[test]
    fn a_recording_containing_the_boundary_gets_a_longer_one() {
        let hostile = format!("noise{BOUNDARY_SEED}noise").into_bytes();
        let body = form_body("whisper-1", &recording(&hostile));
        let boundary = body
            .content_type
            .rsplit("boundary=")
            .next()
            .unwrap()
            .to_owned();
        assert!(
            boundary.len() > BOUNDARY_SEED.len(),
            "a boundary the payload also spells cuts the body in half"
        );
        assert!(!contains(&hostile, boundary.as_bytes()));
    }

    #[test]
    fn an_answer_without_text_is_a_wire_mismatch_naming_the_key() {
        let err = transcription_of(&serde_json::json!({ "duration": 3 })).unwrap_err();
        assert_eq!(err.code(), &AxCode::WireMismatch);
        assert!(err.subject().contains("transcription.text"));
    }

    #[test]
    fn silence_transcribes_to_an_empty_string_and_that_is_an_answer() {
        let text = transcription_of(&serde_json::json!({ "text": "" })).unwrap();
        assert!(text.is_empty());
    }
}
