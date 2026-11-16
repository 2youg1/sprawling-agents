// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The picture that came back

use base64::Engine as _;
use kernel::{AxCode, AxError, ImageType};
use serde_json::Value;

/// The picture that came back, with its two sides read from the bytes
/// rather than from what anybody said they would be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    media: ImageType,
}

impl Shot {
    /// Reads a `browsingContext.captureScreenshot` result.
    ///
    /// # Errors
    /// Refuses a reply with no `data`, data that is not base64, and any
    /// format whose size this build cannot read from the bytes. The last
    /// one is a refusal rather than a guess: the two sides are the only
    /// scale a model has for a picture, and a wrong one is worse than
    /// none.
    pub fn read(result: &Value, media: ImageType) -> Result<Shot, AxError> {
        let encoded = result.get("data").and_then(Value::as_str).ok_or_else(|| {
            AxError::failure(AxCode::WireMismatch, "read a screenshot", "no `data`")
                .with_recovery("check the driver's protocol version")
        })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| {
                AxError::failure(AxCode::WireMismatch, "read a screenshot", err.to_string())
                    .with_recovery("check the driver's protocol version")
            })?;
        let (width, height) = size_of(&bytes, media)?;
        Ok(Shot {
            bytes,
            width,
            height,
            media,
        })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn media(&self) -> ImageType {
        self.media
    }
}

/// The two sides, read from the bytes.
///
/// # Errors
/// Refuses bytes that are not the format they claim to be, and every
/// format except PNG — which is what this build asks for by default and
/// the only one it can measure without a second decoder.
fn size_of(bytes: &[u8], media: ImageType) -> Result<(u32, u32), AxError> {
    if media != ImageType::Png {
        return Err(AxError::failure(
            AxCode::WireMismatch,
            "read a screenshot",
            format!("{} carries no size this build can read", media.mime()),
        )
        .with_recovery("ask for png, which is what a screenshot is asked for by default"));
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let info = decoder.read_header_info().map_err(|err| {
        AxError::failure(AxCode::WireMismatch, "read a screenshot", err.to_string())
            .with_recovery("the driver said png; these bytes are not png")
    })?;
    Ok((info.width, info.height))
}
