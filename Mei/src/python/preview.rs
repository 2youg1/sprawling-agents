// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Cursor;

use image::{ImageFormat, ImageReader, Limits};

pub(super) fn thumbnail(media: &str, data: &[u8]) -> Option<String> {
    let format = match media {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        _ => return None,
    };
    let mut reader = ImageReader::with_format(Cursor::new(data), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    let picture = match reader.decode() {
        Ok(picture) => picture,
        Err(_) => return None,
    };
    let tiles = picture.thumbnail(56, 36).to_luma_alpha8();
    let mut out = String::new();
    for (row_number, row) in tiles.rows().enumerate() {
        if row_number % 2 != 0 {
            continue;
        }
        for pixel in row {
            let [light, alpha] = pixel.0;
            let level = u32::from(light).saturating_mul(u32::from(alpha));
            out.push(match level {
                0..=13_004 => ' ',
                13_005..=26_009 => '░',
                26_010..=39_014 => '▒',
                39_015..=52_019 => '▓',
                _ => '█',
            });
        }
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn real_png_and_jpeg_have_a_grayscale_terminal_preview() {
        let samples =
            image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(12, 12, |x, _| {
                image::Rgb(if x < 6 { [255, 255, 255] } else { [40, 40, 40] })
            }));
        for (mime, format) in [
            ("image/png", ImageFormat::Png),
            ("image/jpeg", ImageFormat::Jpeg),
        ] {
            let mut bytes = Cursor::new(Vec::new());
            samples.write_to(&mut bytes, format).unwrap();
            let rendered = thumbnail(mime, bytes.get_ref()).unwrap();
            assert!(rendered.contains('█'));
            assert!(rendered.contains(' '));
        }
    }
}
