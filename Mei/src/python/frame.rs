// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Read;

use base64::Engine as _;
use serde_json::{Map, Value};

use super::{IMAGE_CALL_LIMIT, IMAGE_MIB};
use crate::error::{Code, Diagnostic, Error, Result};

const IMAGE_BYTES: usize = IMAGE_MIB * 1024 * 1024;

pub struct Image {
    pub(super) media_type: &'static str,
    pub(super) data: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Capture {
    File(String),
    Text(String),
}

pub struct Frame {
    pub capture: Capture,
    pub images: Vec<Image>,
    pub failed: bool,
}

fn failure(subject: impl Into<String>, value: impl Into<String>) -> Error {
    Diagnostic::new(
        Code::Python,
        "parse a Python response frame",
        subject,
        "restart Mei with its embedded Python bootstrap and keep the private control channel intact",
    )
    .with_path("python.response")
    .with_value(value)
    .into()
}

fn field(input: &mut impl Read, target: &mut [u8], name: &'static str) -> Result<()> {
    input
        .read_exact(target)
        .map_err(|err| failure(format!("read {name}: {err}"), target.len().to_string()))
}

fn image(input: &mut impl Read) -> Result<Image> {
    let mut header = [0; 5];
    field(input, &mut header, "image header")?;
    let [kind, a, b, c, d] = header;
    let media_type = match kind {
        1 => "image/png",
        2 => "image/jpeg",
        3 => "image/gif",
        4 => "image/webp",
        _ => return Err(failure("unsupported image format", kind.to_string())),
    };
    let length = u32::from_le_bytes([a, b, c, d]);
    let size = usize::try_from(length)
        .map_err(|err| failure(format!("read image length: {err}"), length.to_string()))?;
    if !(1..=IMAGE_BYTES).contains(&size) {
        return Err(failure(
            "image length is outside the allowed range",
            length.to_string(),
        ));
    }
    let mut data = vec![0; size];
    field(input, &mut data, "image bytes")?;
    Ok(Image { media_type, data })
}

fn capture(input: &mut impl Read, length: u8) -> Result<Capture> {
    if length == 0 {
        let mut length = [0; 2];
        field(input, &mut length, "inline capture length")?;
        let size = u16::from_le_bytes(length);
        if u64::from(size) > crate::capture::INLINE_BYTES {
            return Err(failure(
                "inline capture exceeds the allowed length",
                size.to_string(),
            ));
        }
        let mut bytes = vec![0; usize::from(size)];
        field(input, &mut bytes, "inline capture")?;
        return String::from_utf8(bytes).map(Capture::Text).map_err(|err| {
            failure(
                format!("inline capture is not UTF-8: {err}"),
                size.to_string(),
            )
        });
    }
    let mut bytes = vec![0; usize::from(length)];
    field(input, &mut bytes, "capture file name")?;
    if !bytes.is_ascii() {
        return Err(failure(
            "capture file name is not ASCII",
            format!("{bytes:?}"),
        ));
    }
    String::from_utf8(bytes)
        .map(Capture::File)
        .map_err(|err| failure(format!("read capture file name: {err}"), length.to_string()))
}

pub fn read(input: &mut impl Read) -> Result<Frame> {
    let mut header = [0; 3];
    field(input, &mut header, "response header")?;
    let [length, failed, count] = header;
    let failed = match failed {
        0 => false,
        1 => true,
        _ => return Err(failure("failed is not a boolean", failed.to_string())),
    };
    let count = usize::from(count);
    if count > IMAGE_CALL_LIMIT {
        return Err(failure("too many pictures", count.to_string()));
    }
    let capture = capture(input, length)?;
    let images = (0..count).map(|_| image(input)).collect::<Result<_>>()?;
    Ok(Frame {
        capture,
        images,
        failed,
    })
}

pub fn content(text: String, images: Vec<Image>) -> Value {
    if images.is_empty() {
        return Value::String(text);
    }
    let mut blocks = Vec::new();
    if !text.is_empty() {
        blocks.push(Value::Object(Map::from_iter([
            ("type".to_owned(), Value::from("text")),
            ("text".to_owned(), Value::String(text)),
        ])));
    }
    blocks.extend(images.into_iter().map(|image| {
        let source = Map::from_iter([
            ("type".to_owned(), Value::from("base64")),
            ("media_type".to_owned(), Value::from(image.media_type)),
            (
                "data".to_owned(),
                Value::String(base64::engine::general_purpose::STANDARD.encode(image.data)),
            ),
        ]);
        Value::Object(Map::from_iter([
            ("type".to_owned(), Value::from("image")),
            ("source".to_owned(), Value::Object(source)),
        ]))
    }));
    Value::Array(blocks)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn text_only_frames_keep_the_legacy_string_result() {
        let mut input = b"\x07\x00\x00out.txt".as_slice();
        let frame = read(&mut input).unwrap();
        assert_eq!(frame.capture, Capture::File("out.txt".to_owned()));
        assert!(!frame.failed);
        assert_eq!(
            content("answer".to_owned(), frame.images),
            Value::String("answer".to_owned())
        );
        assert!(input.is_empty());
    }

    #[test]
    fn inline_frames_carry_complete_utf8_and_leave_the_next_frame_aligned() {
        let mut input = b"\x00\x00\x00\x03\x00a\x00b\x00\x01\x00\x00\x00".as_slice();
        assert_eq!(
            read(&mut input).unwrap().capture,
            Capture::Text("a\0b".to_owned())
        );
        let empty = read(&mut input).unwrap();
        assert!(empty.failed);
        assert_eq!(empty.capture, Capture::Text(String::new()));
        assert!(input.is_empty());
    }

    #[test]
    fn images_become_anthropic_blocks_without_base64_in_text() {
        let bytes = b"\x07\x01\x02out.txt\x01\x05\x00\x00\x00image\x04\x03\x00\x00\x00raw";
        let mut input = bytes.as_slice();
        let frame = read(&mut input).unwrap();
        assert!(frame.failed);
        assert_eq!(
            content("opened".to_owned(), frame.images),
            json!([
                {"type": "text", "text": "opened"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "aW1hZ2U="}},
                {"type": "image", "source": {"type": "base64", "media_type": "image/webp", "data": "cmF3"}},
            ])
        );
        assert!(input.is_empty());
    }

    #[test]
    fn malformed_frames_and_media_are_refused() {
        let bad: &[&[u8]] = &[
            b"",
            b"\x00\x00\x00\x01\x40",
            b"\x00\x00\x00\x02\x00x",
            b"\x00\x00\x00\x01\x00\xff",
            b"\x01\x00",
            b"\x00\x00\x00",
            b"\x01\x02\x00x",
            b"\x01\x00\x03x",
            b"\x01\x00\x00\xff",
            b"\x01\x00\x00",
            b"\x01\x00\x01x\x05\x01\x00\x00\x00x",
            b"\x01\x00\x01x\x01\x00\x00\x00\x00",
            b"\x01\x00\x01x\x01\x01\x00\x50\x00",
            b"\x01\x00\x01x\x01\x01\x00\x00\x00",
        ];
        for mut bytes in bad.iter().copied() {
            assert!(read(&mut bytes).is_err());
        }
    }
}
