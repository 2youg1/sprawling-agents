// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A screenshot: what was asked for, and what came back.
//!
//! Every number a caller gives is an integer, including the two the
//! protocol spells as fractions. Quality and scale arrive as percentages
//! and become JSON numbers by formatting digits, so no floating-point
//! value exists in this crate at all — which is what lets a screenshot
//! request sit in a ledger payload beside everything else.

use base64::Engine as _;
use kernel::{AxCode, AxError, ImageType, Payload};
use serde_json::{Value, json};

use crate::port::Frame;
use crate::session::{ContextId, Session};

/// The rectangle of the page a screenshot covers, in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clip {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// What one screenshot was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotRequest {
    clip: Option<Clip>,
    format: ImageType,
    /// 0..=100. Read by the two lossy formats and ignored by PNG, which
    /// is what the protocol does with it too.
    quality: Option<u8>,
    /// A percentage of the device pixel ratio; 100 leaves it alone.
    scale: Option<u32>,
}

impl Default for ShotRequest {
    fn default() -> ShotRequest {
        ShotRequest {
            clip: None,
            format: ImageType::Png,
            quality: None,
            scale: None,
        }
    }
}

fn integer(args: &Payload, field: &str) -> Result<Option<u32>, AxError> {
    match args.as_map().get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let raw = value.as_u64().ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    format!("`{field}` is not a whole number"),
                )
                .with_recovery("pass whole numbers; this city keeps floats out of payloads")
            })?;
            let narrowed = u32::try_from(raw).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    format!("`{field}` is larger than a page can be"),
                )
                .with_recovery("pass a pixel count that fits in 32 bits")
            })?;
            Ok(Some(narrowed))
        }
    }
}

fn required(field: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a screenshot request",
        format!("a clip needs `{field}`"),
    )
    .with_recovery("a clip is x, y, width and height together")
}

/// A JSON number built out of digits rather than out of a float.
///
/// # Errors
/// Refuses a percentage this build cannot spell, which the two callers
/// below cannot produce.
fn ratio(percent: u32) -> Result<Value, AxError> {
    let whole = percent
        .checked_div(100)
        .ok_or_else(|| unreadable(percent))?;
    let rest = percent
        .checked_rem(100)
        .ok_or_else(|| unreadable(percent))?;
    serde_json::from_str(&format!("{whole}.{rest:02}")).map_err(|_| unreadable(percent))
}

fn unreadable(percent: u32) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a screenshot request",
        format!("{percent} is not a percentage this build can spell"),
    )
    .with_recovery("pass a percentage between 0 and 1000")
}

impl ShotRequest {
    /// Reads the four optional fields of a `screenshot` call.
    ///
    /// # Errors
    /// Refuses a clip missing one of its four sides, a format this city
    /// does not carry, and any number that is not whole.
    pub fn read(args: &Payload) -> Result<ShotRequest, AxError> {
        let clip = match args.as_map().get("clip") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let inner = Payload::new(value.as_object().ok_or_else(|| required("x"))?.clone())?;
                Some(Clip {
                    x: integer(&inner, "x")?.ok_or_else(|| required("x"))?,
                    y: integer(&inner, "y")?.ok_or_else(|| required("y"))?,
                    width: integer(&inner, "width")?.ok_or_else(|| required("width"))?,
                    height: integer(&inner, "height")?.ok_or_else(|| required("height"))?,
                })
            }
        };
        let format = match args.as_map().get("format").and_then(Value::as_str) {
            None | Some("png") => ImageType::Png,
            Some("jpeg") => ImageType::Jpeg,
            Some("webp") => ImageType::Webp,
            Some(other) => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    other.to_owned(),
                )
                .with_recovery("one of png, jpeg, webp"));
            }
        };
        let quality = match integer(args, "quality")? {
            None => None,
            Some(percent) => Some(u8::try_from(percent.min(100)).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    "quality is a percentage",
                )
                .with_recovery("pass 0 to 100")
            })?),
        };
        Ok(ShotRequest {
            clip,
            format,
            quality,
            scale: integer(args, "scale")?,
        })
    }

    #[must_use]
    pub fn format(&self) -> ImageType {
        self.format
    }

    /// The frames one screenshot needs.
    ///
    /// A scale other than 100 changes the device pixel ratio first,
    /// because the protocol has no per-capture scale and a caller who
    /// asked for one wants more pixels rather than a bigger file.
    ///
    /// # Errors
    /// Propagates the frames' own refusals.
    pub(crate) fn frames(
        &self,
        session: &mut Session,
        context: &ContextId,
    ) -> Result<Vec<Frame>, AxError> {
        let mut frames = Vec::new();
        if let Some(scale) = self.scale.filter(|percent| *percent != 100) {
            frames.push(session.frame(
                "browsingContext.setViewport",
                json!({ "context": context.as_str(), "devicePixelRatio": ratio(scale)? }),
            )?);
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "context".to_owned(),
            Value::String(context.as_str().to_owned()),
        );
        let mut format = serde_json::Map::new();
        format.insert(
            "type".to_owned(),
            Value::String(self.format.mime().to_owned()),
        );
        if let Some(quality) = self.quality {
            format.insert("quality".to_owned(), ratio(u32::from(quality))?);
        }
        params.insert("format".to_owned(), Value::Object(format));
        if let Some(clip) = self.clip {
            params.insert(
                "clip".to_owned(),
                json!({
                    "type": "box",
                    "x": clip.x,
                    "y": clip.y,
                    "width": clip.width,
                    "height": clip.height,
                }),
            );
        }
        frames.push(session.frame("browsingContext.captureScreenshot", Value::Object(params))?);
        Ok(frames)
    }
}

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
