// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Error, Result};

pub struct Image {
    pub(super) media_type: String,
    pub(super) data: String,
}

pub struct Frame {
    pub capture: String,
    pub images: Vec<Image>,
    pub failed: bool,
}

fn failure(subject: impl Into<String>, value: impl Into<String>) -> Error {
    Diagnostic::new(
        Code::Python,
        "parse a Python response frame",
        subject,
        "keep the private channel limited to {capture, images, failed} with base64 PNG/JPEG/GIF/WebP images",
    )
    .with_path("python.response")
    .with_value(value)
    .into()
}

fn image(value: &Value) -> Result<Image> {
    let media_type = value
        .get("media_type")
        .and_then(Value::as_str)
        .filter(|kind| {
            matches!(
                *kind,
                "image/png" | "image/jpeg" | "image/gif" | "image/webp"
            )
        })
        .ok_or_else(|| failure("image media_type is unsupported", value.to_string()))?;
    let data = value
        .get("data")
        .and_then(Value::as_str)
        .filter(|data| !data.is_empty())
        .ok_or_else(|| failure("image data is missing or empty", value.to_string()))?;
    Ok(Image {
        media_type: media_type.to_owned(),
        data: data.to_owned(),
    })
}

pub fn parse(line: &str) -> Result<Frame> {
    let value: Value =
        serde_json::from_str(line).map_err(|err| failure(err.to_string(), line.to_owned()))?;
    let capture = value
        .get("capture")
        .and_then(Value::as_str)
        .filter(|capture| !capture.is_empty())
        .ok_or_else(|| failure("capture is missing or empty", value.to_string()))?;
    let images = value
        .get("images")
        .and_then(Value::as_array)
        .ok_or_else(|| failure("images is missing or is not an array", value.to_string()))?
        .iter()
        .map(image)
        .collect::<Result<Vec<_>>>()?;
    Ok(Frame {
        capture: capture.to_owned(),
        images,
        failed: value
            .get("failed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub fn content(text: String, images: Vec<Image>) -> Value {
    if images.is_empty() {
        return Value::String(text);
    }
    let mut blocks = Vec::new();
    if !text.is_empty() {
        blocks.push(json!({"type": "text", "text": text}));
    }
    blocks.extend(images.into_iter().map(|image| {
        json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": image.media_type,
                "data": image.data,
            }
        })
    }));
    Value::Array(blocks)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn text_only_frames_keep_the_legacy_string_result() {
        let frame = parse("{\"capture\":\"out.txt\",\"images\":[]}").unwrap();
        assert_eq!(
            content("answer".to_owned(), frame.images),
            Value::String("answer".to_owned())
        );
    }

    #[test]
    fn images_become_anthropic_blocks_without_base64_in_text() {
        let frame = parse("{\"capture\":\"out.txt\",\"images\":[{\"media_type\":\"image/png\",\"data\":\"aW1hZ2U=\"}]}").unwrap();
        assert_eq!(
            content("opened".to_owned(), frame.images),
            json!([
                {"type": "text", "text": "opened"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "aW1hZ2U="}}
            ])
        );
    }

    #[test]
    fn malformed_frames_and_media_are_refused() {
        for line in [
            "{}",
            "{\"capture\":\"out.txt\",\"images\":[{ \"media_type\":\"text/plain\", \"data\":\"eA==\"}]}",
        ] {
            assert!(parse(line).is_err());
        }
    }
}
