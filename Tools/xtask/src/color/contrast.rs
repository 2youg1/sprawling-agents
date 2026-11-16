// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{GRAY_CHROMA, HUE_AXIS};

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

pub(super) fn bronze_tier(px: u16, weight: u16, body: bool) -> Option<u16> {
    let rows: &[(u16, &[(u16, u16)])] = if body { &BRONZE_BODY } else { &BRONZE_CONTENT };
    rows.iter()
        .filter(|(_, mins)| {

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

pub(super) fn apca_lc(text: u16, surface: u16) -> f64 {
    lc_between(
        screen_luminance(axis_encoded(text)),
        screen_luminance(axis_encoded(surface)),
    )
}

pub(super) fn apca_lc_over(text: u16, fill: u16, backdrop: u16, percent: u16) -> f64 {
    let alpha = f64::from(percent.min(100)) / 100.0;
    let mut mixed = [0.0; 3];
    for (slot, (top, bottom)) in mixed
        .iter_mut()
        .zip(axis_encoded(fill).into_iter().zip(axis_encoded(backdrop)))
    {
        *slot = alpha * top + (1.0 - alpha) * bottom;
    }
    lc_between(
        screen_luminance(axis_encoded(text)),
        screen_luminance(mixed),
    )
}

fn lc_between(text_y: f64, surface_y: f64) -> f64 {
    let text_y = soft_clamp(text_y);
    let surface_y = soft_clamp(surface_y);
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

fn axis_encoded(lightness: u16) -> [f64; 3] {
    let l = f64::from(lightness) / 1000.0;
    let chroma = f64::from(GRAY_CHROMA) / 1000.0;
    let hue = f64::from(HUE_AXIS).to_radians();
    let (a, b) = (chroma * hue.cos(), chroma * hue.sin());
    let long = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let medium = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let short = (l - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);
    [
        4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short,
        -1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short,
        -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701_0 * short,
    ]
    .map(|channel| {
        let clipped = channel.clamp(0.0, 1.0);
        if clipped <= 0.003_130_8 {
            12.92 * clipped
        } else {
            1.055 * clipped.powf(1.0 / 2.4) - 0.055
        }
    })
}

fn screen_luminance(encoded: [f64; 3]) -> f64 {
    let coefficients = [0.212_672_9, 0.715_152_2, 0.072_175_0];
    encoded
        .into_iter()
        .zip(coefficients)
        .map(|(channel, weight)| weight * ((channel * 255.0).round() / 255.0).powf(2.4))
        .sum()
}

fn soft_clamp(y: f64) -> f64 {
    if y > 0.022 {
        y
    } else {
        y + (0.022 - y).powf(1.414)
    }
}
