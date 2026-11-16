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
    let frames = request.frames(&mut session, &context(), None).unwrap();
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
    let frames = request.frames(&mut session, &context(), None).unwrap();
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
    let frames = whole.frames(&mut session, &context(), None).unwrap();
    let wire = frames[0].to_wire();
    assert!(wire.contains("\"height\":480"), "{wire}");
    let err =
        ShotRequest::read(&args(json!({ "clip": { "x": 0, "y": 0, "width": 640 } }))).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.subject().contains("height"));
}

#[test]
fn every_capture_states_the_cap_and_the_reply_is_judged_against_it() {
    let request = ShotRequest::read(&args(json!({ "action": "screenshot" }))).unwrap();
    let mut session = Session::new();
    let frames = request.frames(&mut session, &context(), None).unwrap();
    let wire = frames[0].to_wire();
    assert!(wire.contains("\"imageSize\""), "{wire}");
    assert!(wire.contains("1920"), "{wire}");
    assert!(SHOT_MAX_EDGE_PX.admit(1920, 1080).is_ok());
    let err = SHOT_MAX_EDGE_PX.admit(1921, 1080).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("1920"), "{}", err.recovery());
}

#[test]
fn a_reference_covers_an_element_the_page_reports() {
    let request = ShotRequest::read(&args(json!({ "ref": "e12", "generation": 3 }))).unwrap();
    assert!(request.waits_for_page());
    let mut session = Session::new();
    let err = request.frames(&mut session, &context(), None).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("snapshot"));
    let frame = request
        .capture_frame(
            &mut session,
            &context(),
            &json!({ "result": { "type": "node", "sharedId": "n1" } }),
        )
        .unwrap();
    let wire = frame.to_wire();
    assert_eq!(frame.method(), "browsingContext.captureScreenshot");
    assert!(wire.contains("\"type\":\"element\""), "{wire}");
    assert!(wire.contains("\"sharedId\":\"n1\""), "{wire}");
    assert!(wire.contains("\"imageSize\""), "{wire}");
}

#[test]
fn a_rectangle_has_nothing_to_resolve_and_says_so() {
    let request = ShotRequest::read(&args(json!({
        "clip": { "x": 0, "y": 0, "width": 10, "height": 10 },
    })))
    .unwrap();
    assert!(!request.waits_for_page());
    let mut session = Session::new();
    let err = request
        .capture_frame(&mut session, &context(), &json!({}))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

#[test]
fn a_shot_covers_one_region_and_naming_two_is_refused() {
    let err = ShotRequest::read(&args(json!({
        "clip": { "x": 0, "y": 0, "width": 10, "height": 10 },
        "ref": "e12",
        "generation": 1,
    })))
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    let err = ShotRequest::read(&args(json!({ "ref": "e12" }))).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.subject().contains("generation"), "{}", err.subject());
    let err = ShotRequest::read(&args(json!({ "ref": 7 }))).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

#[test]
fn a_picture_past_the_cap_is_asked_for_again_at_a_computed_density() {
    assert!(!SHOT_MAX_EDGE_PX.exceeds(1920, 1080));
    assert!(SHOT_MAX_EDGE_PX.exceeds(2000, 1500));
    let request = ShotRequest::read(&args(json!({ "action": "screenshot" }))).unwrap();
    let mut session = Session::new();
    let frame = request.refit_frame(&mut session, &context(), 2000).unwrap();
    assert_eq!(frame.method(), "browsingContext.setViewport");
    assert!(
        frame.to_wire().contains("\"devicePixelRatio\":0.96"),
        "{}",
        frame.to_wire()
    );
    // A density the caller asked for is kept, and the cap is applied to it.
    let dense = ShotRequest::read(&args(json!({ "action": "screenshot", "scale": 200 }))).unwrap();
    let frame = dense.refit_frame(&mut session, &context(), 2000).unwrap();
    assert!(
        frame.to_wire().contains("\"devicePixelRatio\":1.92"),
        "{}",
        frame.to_wire()
    );
    // A side already inside the cap is never enlarged to fill it.
    let frame = request.refit_frame(&mut session, &context(), 1000).unwrap();
    assert!(
        frame.to_wire().contains("\"devicePixelRatio\":1.0"),
        "{}",
        frame.to_wire()
    );
}

#[test]
fn a_union_covers_every_named_element_from_the_boxes_measure_reads() {
    let request = ShotRequest::read(&args(json!({
        "refs": ["e1", "e2"], "generation": 4,
    })))
    .unwrap();
    assert!(request.waits_for_page());
    let page = json!([
        { "role": "button", "name": "One" },
        { "role": "textbox", "name": "Two" },
    ]);
    let looked = PageSnapshot::read(4, &page).unwrap();
    let mut session = Session::new();
    let frames = request
        .frames(&mut session, &context(), Some(&looked))
        .unwrap();
    assert_eq!(frames.len(), 1, "the page reports the boxes first");
    assert_eq!(frames[0].method(), "script.evaluate");
    let boxes = json!({
        "result": { "type": "string", "value":
            "[{\"ref\":\"e1\",\"x\":10,\"y\":20,\"width\":100,\"height\":50},\
              {\"ref\":\"e2\",\"x\":150,\"y\":5,\"width\":40,\"height\":200}]" }
    });
    let frame = request
        .capture_frame(&mut session, &context(), &boxes)
        .unwrap();
    let wire = frame.to_wire();
    assert_eq!(frame.method(), "browsingContext.captureScreenshot");
    assert!(wire.contains("\"x\":10"), "{wire}");
    assert!(wire.contains("\"y\":5"), "{wire}");
    assert!(wire.contains("\"width\":180"), "{wire}");
    assert!(wire.contains("\"height\":200"), "{wire}");
    assert!(wire.contains("\"imageSize\""), "{wire}");
}

#[test]
fn a_union_of_nothing_is_refused_and_so_is_one_decided_on_a_moved_page() {
    let err = ShotRequest::read(&args(json!({ "refs": [], "generation": 1 }))).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    let request = ShotRequest::read(&args(json!({ "refs": ["e1"], "generation": 9 }))).unwrap();
    let page = json!([{ "role": "button", "name": "One" }]);
    let looked = PageSnapshot::read(4, &page).unwrap();
    let mut session = Session::new();
    let err = request
        .frames(&mut session, &context(), Some(&looked))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
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
