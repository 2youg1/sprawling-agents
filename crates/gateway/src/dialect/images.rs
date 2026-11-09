// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes behind the pictures one request refers to.
//!
//! A leaf value both dialect families read, in the same position
//! `mismatch` holds: nothing here reaches back up. The translation stays
//! a pure function of data because the bytes arrive as a parameter —
//! whoever is about to send the request resolves each `cas:` locator
//! first, so the same pair of values always produces the same bytes on
//! the wire and a replay can rebuild them.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use base64::Engine as _;
use image::ImageDecoder as _;
use kernel::{AxCode, AxError, ImageRef, Locator};

use crate::mismatch::mismatch;

/// Pictures by locator.
///
/// Keyed by the locator's canonical spelling, which is what makes the
/// iteration order a fact about the content rather than about the run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageBytes(BTreeMap<String, String>);

impl ImageBytes {
    /// Encodes on arrival so the raw allocation can be released before the
    /// next picture is read, and repeated wire positions reuse this text.
    /// Only a picture past the model's documented side ceiling is decoded
    /// and resized; every other picture crosses byte for byte.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when dimensions cannot be read, resizing fails,
    /// or resizing would discard animation frames.
    pub fn insert(
        &mut self,
        picture: &ImageRef,
        bytes: Vec<u8>,
        max_side: Option<NonZeroU32>,
    ) -> Result<(), AxError> {
        let bytes = match max_side {
            Some(limit) => resize_picture(picture, bytes, limit)?,
            None => bytes,
        };
        self.0.insert(
            picture.locator.to_string(),
            base64::engine::general_purpose::STANDARD.encode(bytes),
        );
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The picture as both wires carry it: standard base64, padded.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` when nobody resolved this locator. Sending the
    /// request without the picture would leave the model looking at a
    /// gap it was told about, so the whole call is refused instead.
    pub(crate) fn encoded(&self, at: &Locator) -> Result<&str, AxError> {
        self.0
            .get(&at.to_string())
            .map(String::as_str)
            .ok_or_else(|| {
                mismatch(
                    "content.image.locator",
                    "no bytes were resolved for this picture",
                )
            })
    }
}

/// A picture that failed provider-side preparation. The original CAS
/// object is untouched; the caller gets a local, actionable refusal.
fn invalid_picture(picture: &ImageRef, detail: impl std::fmt::Display, max_side: u32) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "prepare a picture for this model",
        format!(
            "{}: cannot fit its {}x{} sides within {max_side} pixels: {detail}",
            picture.locator, picture.width, picture.height
        ),
    )
    .with_recovery(
        "pass a complete PNG, JPEG, WebP or GIF whose longest side fits this model; for an \
         animation, choose a static frame before dispatch",
    )
}

/// Whether this format carries more than one frame. Only an oversized
/// picture reaches this question; resizing one must not silently flatten it.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "image::ImageFormat is upstream and non-exhaustive; unsupported formats fail at decode"
)]
fn is_animated(format: image::ImageFormat, bytes: &[u8]) -> image::ImageResult<bool> {
    use image::AnimationDecoder as _;
    match format {
        image::ImageFormat::Png => {
            image::codecs::png::PngDecoder::new(std::io::Cursor::new(bytes))?.is_apng()
        }
        image::ImageFormat::WebP => {
            Ok(image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(bytes))?.has_animation())
        }
        image::ImageFormat::Gif => {
            let mut decoder = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))?;
            let limits = image::Limits::default();
            decoder.set_limits(limits.clone())?;
            let (width, height) = decoder.dimensions();
            limits
                .clone()
                .reserve_buffer(width, height, image::ColorType::Rgba8)?;
            let mut frames = decoder.into_frames();
            frames.next().transpose()?;
            Ok(frames.next().transpose()?.is_some())
        }
        image::ImageFormat::Jpeg => Ok(false),
        _ => Ok(false),
    }
}

/// Original bytes when dimensions fit, otherwise the same format within
/// `max_side`. EXIF
/// orientation becomes pixels before resizing, so dropping that metadata
/// cannot rotate the picture the model receives.
fn resize_picture(
    picture: &ImageRef,
    bytes: Vec<u8>,
    max_side: NonZeroU32,
) -> Result<Vec<u8>, AxError> {
    let reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|err| invalid_picture(picture, err, max_side.get()))?;
    let format = reader
        .format()
        .ok_or_else(|| invalid_picture(picture, "its format is not recognised", max_side.get()))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|err| invalid_picture(picture, err, max_side.get()))?;
    let (width, height) = decoder.dimensions();
    if width.max(height) <= max_side.get() {
        drop(decoder);
        return Ok(bytes);
    }
    if is_animated(format, &bytes).map_err(|err| invalid_picture(picture, err, max_side.get()))? {
        return Err(invalid_picture(
            picture,
            "resizing would discard its later animation frames",
            max_side.get(),
        ));
    }
    let orientation = decoder
        .orientation()
        .map_err(|err| invalid_picture(picture, err, max_side.get()))?;
    let mut decoded = image::DynamicImage::from_decoder(decoder)
        .map_err(|err| invalid_picture(picture, err, max_side.get()))?;
    decoded.apply_orientation(orientation);
    let resized = decoded.resize(
        max_side.get(),
        max_side.get(),
        image::imageops::FilterType::Triangle,
    );
    let mut encoded = Vec::new();
    resized
        .write_to(&mut std::io::Cursor::new(&mut encoded), format)
        .map_err(|err| invalid_picture(picture, err, max_side.get()))?;
    Ok(encoded)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "generated fixtures must encode and the asserted results must decode"
)]
mod tests {
    use super::*;

    /// A tiny frame cannot make an oversized logical GIF screen safe to allocate.
    #[test]
    fn gif_canvas_budget_is_checked_before_frame_iteration() {
        let mut bytes = Vec::new();
        image::codecs::gif::GifEncoder::new(&mut bytes)
            .encode_frames([image::Frame::new(image::RgbaImage::new(1, 1))])
            .expect("valid generated image fixture");
        bytes
            .get_mut(6..10)
            .expect("GIF logical screen dimensions")
            .copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
        let picture = ImageRef {
            locator: Locator::cas(kernel::B3Hash::digest(&bytes)),
            media_type: kernel::ImageType::Gif,
            width: 1,
            height: 1,
        };
        let err = ImageBytes::default()
            .insert(&picture, bytes, NonZeroU32::new(8192))
            .expect_err("oversized canvas must be refused");
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.to_string().contains("Memory limit"), "{err}");
    }

    /// A static GIF past the side ceiling still follows the existing resize path.
    #[test]
    fn a_static_gif_within_the_allocation_budget_can_be_resized() {
        let mut bytes = Vec::new();
        image::codecs::gif::GifEncoder::new(&mut bytes)
            .encode_frames([image::Frame::new(image::RgbaImage::new(2, 16))])
            .expect("valid generated image fixture");
        let picture = ImageRef {
            locator: Locator::cas(kernel::B3Hash::digest(&bytes)),
            media_type: kernel::ImageType::Gif,
            width: 1,
            height: 1,
        };
        let mut images = ImageBytes::default();
        images
            .insert(&picture, bytes, NonZeroU32::new(8))
            .expect("valid generated image fixture");
        let sent = base64::engine::general_purpose::STANDARD
            .decode(
                images
                    .encoded(&picture.locator)
                    .expect("resolved fixture locator"),
            )
            .expect("valid generated image fixture");
        let decoded = image::load_from_memory(&sent).expect("valid generated image fixture");
        assert_eq!((decoded.width(), decoded.height()), (1, 8));
    }
}
