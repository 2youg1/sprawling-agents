// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Pixels scaled by a percentage and encoded as png, jpeg or webp.
//!
//! Nothing here touches a desktop, which is the point: what a caller
//! most easily gets wrong about a screenshot is the size it asked for
//! and the format it named, and both are decided here where they can be
//! checked on a machine with no windows at all.
//!
//! Two facts about the encoders are the outside world's rather than
//! ours, and a caller is told about them rather than left to discover
//! them (desktop-SPEC.md §14). This build's webp encoder is **lossless**,
//! so `quality` does not reach it; and jpeg has no alpha channel, so the
//! window is composited onto white before it is encoded — which is a
//! visible choice, not a silent one.

use crate::refusal::{Refusal, RefusalCode};
use base64::Engine as _;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, RgbImage, RgbaImage};
use serde_json::{Value, json};

/// What `format` may say. Exhaustive, so a fourth format is a change to
/// this enum and to the tool schema in one edit rather than a string
/// that quietly falls through to a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Png,
    Jpeg,
    Webp,
}

impl Format {
    /// # Errors
    /// Refuses a format outside the three the tool schema offers.
    pub(crate) fn parse(named: &str) -> Result<Format, Refusal> {
        match named {
            "png" => Ok(Format::Png),
            "jpeg" => Ok(Format::Jpeg),
            "webp" => Ok(Format::Webp),
            other => Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "capture a window",
                format!("`{other}` is not a format this server writes"),
                "ask for `png`, `jpeg` or `webp`",
            )),
        }
    }

    fn mime(self) -> &'static str {
        match self {
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
            Format::Webp => "image/webp",
        }
    }

    /// Whether `quality` means anything to this encoder. `false` is not
    /// a defect to hide: a caller that asked for quality 10 and got a
    /// lossless file should be told, or it will read the file size as a
    /// failure.
    fn honours_quality(self) -> bool {
        matches!(self, Format::Jpeg)
    }
}

/// The default when a caller names no format: the one that loses
/// nothing, because a model asking for a picture of a window has not
/// asked to be shown a worse one.
pub(crate) const DEFAULT_FORMAT: Format = Format::Png;

/// The default when a caller names no scale: the window's own size.
pub(crate) const DEFAULT_SCALE: u32 = 100;

/// The default jpeg quality, which is what a caller who named `jpeg` and
/// nothing else gets.
pub(crate) const DEFAULT_QUALITY: u8 = 85;

/// What a caller asked for, since the three always travel together and
/// no one of them decides anything alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Wanted {
    pub(crate) format: Format,
    pub(crate) scale: u32,
    pub(crate) quality: u8,
}

impl Default for Wanted {
    fn default() -> Wanted {
        Wanted {
            format: DEFAULT_FORMAT,
            scale: DEFAULT_SCALE,
            quality: DEFAULT_QUALITY,
        }
    }
}

/// Scales and encodes one window's pixels into the answer a caller
/// reads.
///
/// # Errors
/// Refuses a scale that leaves no pixels, and an encoder that will not
/// take these bytes. An encoder failure is this server's own defect
/// rather than the caller's, and the refusal says so instead of blaming
/// the arguments.
pub(crate) fn render(pixels: &RgbaImage, wanted: Wanted) -> Result<Value, Refusal> {
    let (width, height) = super::geometry::scaled(pixels.width(), pixels.height(), wanted.scale)?;
    let scaled = if (width, height) == (pixels.width(), pixels.height()) {
        pixels.clone()
    } else {
        image::imageops::resize(pixels, width, height, FilterType::Triangle)
    };
    let bytes = encode(&scaled, wanted)?;
    Ok(json!({
        "mime": wanted.format.mime(),
        "width": width,
        "height": height,
        "base64": base64::engine::general_purpose::STANDARD.encode(&bytes),
        // What the caller got rather than what it asked for. A `quality`
        // that reached nothing is the kind of fact a reader otherwise
        // learns from a surprising file size.
        "lossless": !wanted.format.honours_quality(),
    }))
}

/// One image, in one format.
fn encode(pixels: &RgbaImage, wanted: Wanted) -> Result<Vec<u8>, Refusal> {
    let (width, height) = (pixels.width(), pixels.height());
    let mut bytes: Vec<u8> = Vec::new();
    let written = match wanted.format {
        Format::Png => PngEncoder::new(&mut bytes).write_image(
            pixels.as_raw(),
            width,
            height,
            ExtendedColorType::Rgba8,
        ),
        Format::Jpeg => {
            let opaque = onto_white(pixels);
            JpegEncoder::new_with_quality(&mut bytes, wanted.quality).write_image(
                opaque.as_raw(),
                width,
                height,
                ExtendedColorType::Rgb8,
            )
        }
        Format::Webp => WebPEncoder::new_lossless(&mut bytes).write_image(
            pixels.as_raw(),
            width,
            height,
            ExtendedColorType::Rgba8,
        ),
    };
    written.map_err(|err| {
        Refusal::new(
            RefusalCode::ToolUnavailable,
            "capture a window",
            format!("this server could not encode the capture: {err}"),
            "ask for `png`, which this server writes without composing anything",
        )
    })?;
    Ok(bytes)
}

/// The window composited onto white, for an encoder with no alpha
/// channel.
///
/// White rather than black: a window is nearly always a document, and a
/// transparent corner rendered black reads as a rendering failure to
/// whoever looks at the picture next.
fn onto_white(pixels: &RgbaImage) -> RgbImage {
    let mut opaque = RgbImage::new(pixels.width(), pixels.height());
    for (x, y, source) in pixels.enumerate_pixels() {
        // `channel * alpha + white * (1 - alpha)`, in whole numbers over
        // a denominator of 255 rather than in floating point: this file
        // is on no decision path, but a ratio spelled as a float is a
        // second way of spelling a ratio, and the city has one.
        let blend = |channel: u8, alpha: u8| {
            let over = u32::from(channel).saturating_mul(u32::from(alpha));
            let clear = u32::from(u8::MAX.saturating_sub(alpha));
            let under = u32::from(u8::MAX).saturating_mul(clear);
            let blended = over
                .saturating_add(under)
                .checked_div(u32::from(u8::MAX))
                .unwrap_or_default();
            u8::try_from(blended).unwrap_or(u8::MAX)
        };
        let [red, green, blue, alpha] = source.0;
        opaque.put_pixel(
            x,
            y,
            image::Rgb([blend(red, alpha), blend(green, alpha), blend(blue, alpha)]),
        );
    }
    opaque
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

    fn window(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, y| {
            let channel = |value: u32| u8::try_from(value.wrapping_rem(256)).unwrap_or_default();
            image::Rgba([channel(x), channel(y), 0x40, 0xFF])
        })
    }

    fn decoded(answer: &Value) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(answer["base64"].as_str().unwrap())
            .unwrap()
    }

    /// Each of the three formats writes something, says what it wrote,
    /// and reports the size it actually produced.
    #[test]
    fn all_three_formats_encode_and_name_themselves() {
        for (format, mime) in [
            (Format::Png, "image/png"),
            (Format::Jpeg, "image/jpeg"),
            (Format::Webp, "image/webp"),
        ] {
            let answer = render(
                &window(64, 48),
                Wanted {
                    format,
                    ..Wanted::default()
                },
            )
            .unwrap();
            assert_eq!(answer["mime"], mime);
            assert_eq!(answer["width"], 64);
            assert_eq!(answer["height"], 48);
            assert!(!decoded(&answer).is_empty(), "{mime} wrote nothing");
        }
    }

    /// The two magic numbers a reader can check by eye, so a format
    /// mislabelled in the `mime` field would be caught here rather than
    /// by whoever opened the file.
    #[test]
    fn the_bytes_are_the_format_the_answer_claims() {
        let png = decoded(&render(&window(8, 8), Wanted::default()).unwrap());
        assert_eq!(&png[..4], b"\x89PNG");
        let jpeg = decoded(
            &render(
                &window(8, 8),
                Wanted {
                    format: Format::Jpeg,
                    ..Wanted::default()
                },
            )
            .unwrap(),
        );
        assert_eq!(&jpeg[..2], b"\xFF\xD8");
        let webp = decoded(
            &render(
                &window(8, 8),
                Wanted {
                    format: Format::Webp,
                    ..Wanted::default()
                },
            )
            .unwrap(),
        );
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
    }

    /// `scale` is a percentage of the original, and the answer reports
    /// the size that came out rather than the size that went in.
    #[test]
    fn a_scale_changes_the_size_and_the_answer_states_the_new_one() {
        let answer = render(
            &window(200, 100),
            Wanted {
                scale: 25,
                ..Wanted::default()
            },
        )
        .unwrap();
        assert_eq!(answer["width"], 50);
        assert_eq!(answer["height"], 25);
        // Scaling to nothing is refused rather than answered with an
        // empty picture.
        let refusal = render(
            &window(8, 8),
            Wanted {
                scale: 5,
                ..Wanted::default()
            },
        )
        .unwrap_err();
        assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
    }

    /// A caller that asked for quality on a lossless encoder is told
    /// that it did not reach anything.
    #[test]
    fn the_answer_says_whether_quality_reached_the_encoder() {
        let lossy = render(
            &window(16, 16),
            Wanted {
                format: Format::Jpeg,
                quality: 10,
                ..Wanted::default()
            },
        )
        .unwrap();
        assert_eq!(lossy["lossless"], false);
        for lossless in [Format::Png, Format::Webp] {
            let answer = render(
                &window(16, 16),
                Wanted {
                    format: lossless,
                    quality: 10,
                    ..Wanted::default()
                },
            )
            .unwrap();
            assert_eq!(answer["lossless"], true, "{lossless:?}");
        }
    }

    /// A transparent window reaches jpeg as white rather than as black,
    /// so a corner nobody drew does not read as a rendering failure.
    #[test]
    fn a_transparent_pixel_reaches_a_jpeg_as_white() {
        let clear = RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
        let composed = onto_white(&clear);
        assert_eq!(composed.get_pixel(0, 0).0, [255, 255, 255]);
        let solid = RgbaImage::from_pixel(8, 8, image::Rgba([10, 20, 30, 255]));
        assert_eq!(onto_white(&solid).get_pixel(0, 0).0, [10, 20, 30]);
    }

    #[test]
    fn a_format_this_server_does_not_write_is_refused_by_name() {
        for named in ["png", "jpeg", "webp"] {
            assert!(Format::parse(named).is_ok());
        }
        let refusal = Format::parse("gif").unwrap_err();
        assert!(
            refusal.as_error()["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("png")
        );
    }
}
