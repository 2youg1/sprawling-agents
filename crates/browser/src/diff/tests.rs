// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Two screenshots, and what a caller is told about the difference.

use super::*;
use base64::Engine as _;
use serde_json::json;

/// A picture whose pixels are decided by `paint`, as a real PNG read
/// back through the production path.
fn shot(width: u32, height: u32, paint: impl Fn(u32, u32) -> u8) -> Shot {
    let mut raw = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let value = paint(x, y);
            raw.extend_from_slice(&[value, value, value, 255]);
        }
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("a header this build wrote");
        writer
            .write_image_data(&raw)
            .expect("pixels this build made");
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Shot::read(&json!({ "data": encoded }), ImageType::Png).expect("bytes this build wrote")
}

fn blank() -> Shot {
    shot(96, 96, |_, _| 255)
}

#[test]
fn two_pictures_of_the_same_page_differ_by_nothing() {
    let difference = diff(&blank(), &blank()).unwrap();
    assert_eq!(difference.changed_q4(), 0);
    assert!(difference.boxes().is_empty(), "nothing moved, so no box");
}

#[test]
fn one_pixel_that_moved_is_reported_inside_one_box() {
    let after = shot(96, 96, |x, y| if (x, y) == (40, 50) { 0 } else { 255 });
    let difference = diff(&blank(), &after).unwrap();
    assert_eq!(
        difference.changed_q4(),
        1,
        "one pixel of 9216 is one ten-thousandth, rounded down"
    );
    assert_eq!(difference.boxes().len(), 1);
    let found = difference.boxes()[0];
    assert!(found.covers(40, 50), "{found:?}");
    assert!(!found.covers(0, 0), "{found:?}");
}

#[test]
fn two_places_that_moved_are_two_boxes_rather_than_one_that_covers_the_gap() {
    let after = shot(96, 96, |x, y| {
        if (x, y) == (4, 4) || (x, y) == (90, 90) {
            0
        } else {
            255
        }
    });
    let difference = diff(&blank(), &after).unwrap();
    assert_eq!(difference.boxes().len(), 2, "{:?}", difference.boxes());
    assert!(difference.boxes()[0].covers(4, 4));
    assert!(difference.boxes()[1].covers(90, 90));
}

#[test]
fn a_page_that_changed_entirely_is_the_whole_of_it() {
    let after = shot(96, 96, |_, _| 0);
    let difference = diff(&blank(), &after).unwrap();
    assert_eq!(difference.changed_q4(), 10_000);
    assert_eq!(difference.boxes().len(), 1);
    assert_eq!(
        difference.boxes()[0],
        Box2 {
            x: 0,
            y: 0,
            width: 96,
            height: 96
        }
    );
}

#[test]
fn two_pictures_of_different_sizes_are_not_compared() {
    let err = diff(&blank(), &shot(64, 96, |_, _| 255)).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("scaler"));
}
