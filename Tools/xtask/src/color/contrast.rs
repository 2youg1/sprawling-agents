// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Clone, Copy)]
pub(super) struct ContrastColour {
    pub(super) lightness: u16,
    pub(super) chroma: u16,
    pub(super) hue: u16,
}

pub(super) type Srgb = [f64; 3];

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

pub(super) fn apca_lc(text: ContrastColour, surface: ContrastColour) -> f64 {
    encoded_contrast(text.encoded_channels(), surface.encoded_channels())
}

pub(super) fn apca_lc_over(
    text: ContrastColour,
    fill: ContrastColour,
    backdrop: ContrastColour,
    percent: u16,
) -> f64 {
    let alpha = f64::from(percent.min(100)) / 100.0;
    encoded_contrast(
        text.encoded_channels(),
        blend_channels(fill.encoded_channels(), backdrop.encoded_channels(), alpha).map(f64::round),
    )
}

pub(super) fn encoded_contrast(text: Srgb, surface: Srgb) -> f64 {
    lc_between(screen_luminance(text), screen_luminance(surface))
}

pub(super) fn blend_channels(top: Srgb, bottom: Srgb, alpha: f64) -> Srgb {
    let [r, g, b] = top;
    let [dr, dg, db] = bottom;
    let under = 1.0 - alpha;
    [
        r * alpha + dr * under,
        g * alpha + dg * under,
        b * alpha + db * under,
    ]
}

fn lc_between(text_y: f64, surface_y: f64) -> f64 {
    let text_y = soft_clamp(text_y);
    let surface_y = soft_clamp(surface_y);
    if (surface_y - text_y).abs() < 0.0005 {
        return 0.0;
    }
    let raw = if surface_y > text_y {
        let sapc = (surface_y.powf(0.56) - text_y.powf(0.57)) * 1.14;
        if sapc < 0.1 { 0.0 } else { sapc - 0.027 }
    } else {
        let sapc = (surface_y.powf(0.65) - text_y.powf(0.62)) * 1.14;
        if sapc > -0.1 { 0.0 } else { sapc + 0.027 }
    };
    raw * 100.0
}

impl ContrastColour {
    pub(super) fn encoded_channels(self) -> Srgb {
        let lightness = f64::from(self.lightness) / 1000.0;
        let angle = f64::from(self.hue).to_radians();
        let chroma = f64::from(self.chroma) / 1000.0;
        let candidate = linear_channels(lightness, chroma, angle);
        let in_gamut = |rgb: Srgb| rgb.into_iter().all(|value| (0.0..=1.0).contains(&value));
        let linear = if in_gamut(candidate) {
            candidate
        } else {
            let (mut lower, mut upper) = (0.0, chroma);
            for _ in 0..16 {
                let middle = f64::midpoint(lower, upper);
                if in_gamut(linear_channels(lightness, middle, angle)) {
                    lower = middle;
                } else {
                    upper = middle;
                }
            }
            linear_channels(lightness, lower, angle)
        };
        linear.map(|channel| {
            let channel = channel.clamp(0.0, 1.0);
            let encoded = if channel <= 0.003_130_8 {
                12.92 * channel
            } else {
                1.055 * channel.powf(1.0 / 2.4) - 0.055
            };
            (encoded * 255.0).round()
        })
    }
}

fn linear_channels(l: f64, chroma: f64, hue: f64) -> Srgb {
    let (a, b) = (chroma * hue.cos(), chroma * hue.sin());
    let long = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let medium = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let short = (l - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);
    [
        4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short,
        -1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short,
        -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701_0 * short,
    ]
}

fn screen_luminance(encoded: Srgb) -> f64 {
    let coefficients = [0.212_672_9, 0.715_152_2, 0.072_175_0];
    encoded
        .into_iter()
        .zip(coefficients)
        .map(|(channel, weight)| weight * (channel / 255.0).powf(2.4))
        .sum()
}

fn soft_clamp(y: f64) -> f64 {
    if y > 0.022 {
        y
    } else {
        y + (0.022 - y).powf(1.414)
    }
}
