// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::refusal::{Refusal, RefusalCode};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, RgbImage, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Png,
    Jpeg,
    Webp,
}

impl Format {
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

    fn honours_quality(self) -> bool {
        matches!(self, Format::Jpeg)
    }
}

pub(crate) const DEFAULT_FORMAT: Format = Format::Png;

pub(crate) const DEFAULT_SCALE: u32 = 100;

pub(crate) const DEFAULT_QUALITY: u8 = 85;

pub(crate) const SCALE_MAX: u32 = 100;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Encoded {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mime: &'static str,

    pub(crate) width: u32,
    pub(crate) height: u32,

    pub(crate) lossless: bool,
}

pub(crate) fn render(pixels: &RgbaImage, wanted: Wanted) -> Result<Encoded, Refusal> {
    let (width, height) = super::geometry::scaled(pixels.width(), pixels.height(), wanted.scale)?;
    let scaled = if (width, height) == (pixels.width(), pixels.height()) {
        pixels.clone()
    } else {
        image::imageops::resize(pixels, width, height, FilterType::Triangle)
    };
    Ok(Encoded {
        bytes: encode(&scaled, wanted)?,
        mime: wanted.format.mime(),
        width,
        height,
        lossless: !wanted.format.honours_quality(),
    })
}

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

fn onto_white(pixels: &RgbaImage) -> RgbImage {
    let mut opaque = RgbImage::new(pixels.width(), pixels.height());
    for (x, y, source) in pixels.enumerate_pixels() {
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
            assert_eq!(answer.mime, mime);
            assert_eq!((answer.width, answer.height), (64, 48));
            assert!(!answer.bytes.is_empty(), "{mime} wrote nothing");
        }
    }

    #[test]
    fn the_bytes_are_the_format_the_answer_claims() {
        let png = render(&window(8, 8), Wanted::default()).unwrap().bytes;
        assert_eq!(&png[..4], b"\x89PNG");
        let jpeg = render(
            &window(8, 8),
            Wanted {
                format: Format::Jpeg,
                ..Wanted::default()
            },
        )
        .unwrap()
        .bytes;
        assert_eq!(&jpeg[..2], b"\xFF\xD8");
        let webp = render(
            &window(8, 8),
            Wanted {
                format: Format::Webp,
                ..Wanted::default()
            },
        )
        .unwrap()
        .bytes;
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
    }

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
        assert_eq!((answer.width, answer.height), (50, 25));

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
        assert!(!lossy.lossless);
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
            assert!(answer.lossless, "{lossless:?}");
        }
    }

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
