// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The measurement half: APCA lightness contrast on the hue axis, and
//! the Bronze Simple Mode size tables a type step is judged against.

use super::{GRAY_CHROMA, HUE_AXIS};

/// The APCA-RC Bronze Simple Mode minimum sizes, transcribed from the
/// published criterion: for each tier, the smallest size each weight may
/// use. Body text and everything else have different tables, which is the
/// whole reason the two are kept apart.
///
/// Lc 30 is deliberately absent. Its published scope is placeholder text,
/// disabled controls and non-text elements - it is not a floor content may
/// fall back to, and treating it as one makes every size pass.
const BRONZE_BODY: [(u16, &[(u16, u16)]); 2] = [
    (90, &[(300, 18), (400, 14)]),
    (75, &[(300, 24), (400, 18), (500, 16), (700, 14)]),
];
const BRONZE_CONTENT: [(u16, &[(u16, u16)]); 4] = [
    (90, &[(400, 12)]),
    (75, &[(400, 15)]),
    (
        60,
        &[
            (200, 48),
            (300, 36),
            (400, 24),
            (500, 21),
            (600, 18),
            (700, 16),
        ],
    ),
    (45, &[(400, 36), (700, 24)]),
];

/// The lowest tier that admits this size at this weight, or `None` when no
/// tier does - which means the step is too small to be content at any
/// contrast, and is a defect in the type scale rather than in the palette.
pub(super) fn bronze_tier(px: u16, weight: u16, body: bool) -> Option<u16> {
    let rows: &[(u16, &[(u16, u16)])] = if body { &BRONZE_BODY } else { &BRONZE_CONTENT };
    rows.iter()
        .filter(|(_, mins)| {
            // A heavier face is legible smaller, so a weight the table does
            // not list takes the heaviest listed weight at or below it, and
            // failing that the lightest listed - which is the strictest
            // reading available.
            let floor = mins
                .iter()
                .filter(|(w, _)| *w <= weight)
                .max_by_key(|(w, _)| *w)
                .or_else(|| mins.iter().min_by_key(|(w, _)| *w));
            floor.is_some_and(|(_, min_px)| px >= *min_px)
        })
        .map(|(tier, _)| *tier)
        .min()
}

/// APCA lightness contrast between two on-axis tokens, both given as
/// lightness in per mille. Reverse polarity (light text on a dark surface)
/// reports negative, and the absolute value is what the tiers compare
/// against.
///
/// Constants are apca-w3 0.1.9 / 0.98G-4g. The 8-bit quantisation is
/// reproduced because a browser displays quantised channels, and the
/// difference decides a boundary case; it is done in floating point so that
/// no lossy integer conversion appears anywhere in this file.
pub(super) fn apca_lc(text: u16, surface: u16) -> f64 {
    let text_y = soft_clamp(axis_luminance(text));
    let surface_y = soft_clamp(axis_luminance(surface));
    if (surface_y - text_y).abs() < 0.0005 {
        return 0.0;
    }
    let raw = if surface_y > text_y {
        let sapc = (surface_y.powf(0.56) - text_y.powf(0.57)) * 1.14;
        if sapc < 0.001 { 0.0 } else { sapc - 0.027 }
    } else {
        let sapc = (surface_y.powf(0.65) - text_y.powf(0.62)) * 1.14;
        if sapc > -0.001 { 0.0 } else { sapc + 0.027 }
    };
    (raw * 100.0).abs()
}

/// Screen luminance of an on-axis token: OKLCH through OKLab to linear
/// sRGB, quantised to eight bits, then APCA's own simple transfer curve.
fn axis_luminance(lightness: u16) -> f64 {
    let l = f64::from(lightness) / 1000.0;
    let chroma = f64::from(GRAY_CHROMA) / 1000.0;
    let hue = f64::from(HUE_AXIS).to_radians();
    let (a, b) = (chroma * hue.cos(), chroma * hue.sin());
    let long = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let medium = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let short = (l - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);
    let linear = [
        4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short,
        -1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short,
        -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701_0 * short,
    ];
    let coefficients = [0.212_672_9, 0.715_152_2, 0.072_175_0];
    let mut y = 0.0;
    for (channel, weight) in linear.into_iter().zip(coefficients) {
        let clipped = channel.clamp(0.0, 1.0);
        let encoded = if clipped <= 0.003_130_8 {
            12.92 * clipped
        } else {
            1.055 * clipped.powf(1.0 / 2.4) - 0.055
        };
        let quantised = (encoded * 255.0).round() / 255.0;
        y += weight * quantised.powf(2.4);
    }
    y
}

/// APCA lifts the darkest values so that near-black pairs do not report
/// more contrast than an eye finds there.
fn soft_clamp(y: f64) -> f64 {
    if y > 0.022 {
        y
    } else {
        y + (0.022 - y).powf(1.414)
    }
}
