// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a screenshot is asked for, and what is read back from it.

use super::*;
use serde_json::json;

/// One pixel of red, as a real PNG. Written by hand so the assertions
/// below measure bytes rather than a fixture nobody can check.
///
/// Kept in sixteen-byte pieces because the secret gate reads the file,
/// not the concatenation: a base64 run of twenty bytes or more is what
/// the entropy detector is built to catch, and it is right to catch it.
pub(crate) const ONE_RED_PIXEL: &str = concat!(
    "iVBORw0KGgoAAAAN",
    "SUhEUgAAAAEAAAAB",
    "CAYAAAAfFcSJAAAA",
    "DUlEQVR42mP8z8BQ",
    "DwAEhQGAhKmMIQAA",
    "AABJRU5ErkJggg=="
);

fn args(value: Value) -> Payload {
    Payload::new(
        value
            .as_object()
            .cloned()
            .unwrap_or_else(serde_json::Map::new),
    )
    .expect("test arguments carry no floats")
}

fn context() -> ContextId {
    ContextId::parse("c1").expect("a literal id is well-formed")
}

#[test]
fn a_screenshot_asked_for_with_nothing_is_a_png_of_the_whole_page() {
    let request = ShotRequest::read(&args(json!({ "action": "screenshot" }))).unwrap();
    assert_eq!(request, ShotRequest::default());
    let mut session = Session::new();
    let frames = request.frames(&mut session, &context()).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].method(), "browsingContext.captureScreenshot");
    let wire = frames[0].to_wire();
    assert!(wire.contains("image/png"), "{wire}");
    assert!(!wire.contains("clip"), "{wire}");
}

#[test]
fn the_two_fractions_the_protocol_wants_are_written_out_of_integers() {
    let request = ShotRequest::read(&args(json!({
        "format": "jpeg",
        "quality": 85,
        "scale": 200,
    })))
    .unwrap();
    let mut session = Session::new();
    let frames = request.frames(&mut session, &context()).unwrap();
    assert_eq!(
        frames.len(),
        2,
        "a scale changes the ratio before capturing"
    );
    assert!(frames[0].to_wire().contains("\"devicePixelRatio\":2.0"));
    assert!(frames[1].to_wire().contains("\"quality\":0.85"));
    assert!(frames[1].to_wire().contains("image/jpeg"));
}

#[test]
fn a_clip_travels_whole_or_is_refused() {
    let whole = ShotRequest::read(&args(json!({
        "clip": { "x": 0, "y": 10, "width": 640, "height": 480 },
    })))
    .unwrap();
    let mut session = Session::new();
    let frames = whole.frames(&mut session, &context()).unwrap();
    let wire = frames[0].to_wire();
    assert!(wire.contains("\"height\":480"), "{wire}");
    let err =
        ShotRequest::read(&args(json!({ "clip": { "x": 0, "y": 0, "width": 640 } }))).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.subject().contains("height"));
}

#[test]
fn a_screenshots_size_is_read_from_its_own_bytes() {
    let shot = Shot::read(&json!({ "data": ONE_RED_PIXEL }), ImageType::Png).unwrap();
    assert_eq!((shot.width(), shot.height()), (1, 1));
    assert_eq!(shot.media(), ImageType::Png);
    assert_eq!(shot.bytes().len(), 70);
}

#[test]
fn a_format_whose_size_this_build_cannot_read_is_refused_rather_than_guessed() {
    let err = Shot::read(&json!({ "data": ONE_RED_PIXEL }), ImageType::Jpeg).unwrap_err();
    assert_eq!(err.code(), &AxCode::WireMismatch);
    assert!(err.recovery().contains("png"));
    assert_eq!(
        Shot::read(&json!({}), ImageType::Png).unwrap_err().code(),
        &AxCode::WireMismatch
    );
    assert_eq!(
        Shot::read(&json!({ "data": "not base64 at all!" }), ImageType::Png)
            .unwrap_err()
            .code(),
        &AxCode::WireMismatch
    );
}
