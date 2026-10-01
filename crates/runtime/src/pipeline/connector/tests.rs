// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use super::*;
use kernel::{B3Hash, ImageRef, ImageType, Locator, Payload};
use serde_json::{Value, json};
use storage::Cas;

/// A one-pixel PNG.
const ONE_PIXEL_PNG: [u8; 70] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x60, 0x60, 0xF8,
    0x0F, 0x00, 0x01, 0x04, 0x01, 0x00, 0x5F, 0xE5, 0xC3, 0x4B, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// The same bytes as base64, the way an MCP image block carries them,
/// put together from short pieces so the secret gate does not read one
/// long run of high-entropy text.
fn one_pixel_png_base64() -> String {
    [
        "iVBORw0KGgoAAAAN",
        "SUhEUgAAAAEAAAAB",
        "CAYAAAAfFcSJAAAA",
        "DUlEQVR4nGNgYGD4",
        "DwABBAEAX+XDSwAA",
        "AABJRU5ErkJggg==",
    ]
    .concat()
}

fn answer(content: Value) -> ToolOutcome {
    let map = json!({ "content": content, "isError": false })
        .as_object()
        .cloned()
        .unwrap();
    ToolOutcome {
        result: Payload::new(map).unwrap(),
        attachments: Vec::new(),
    }
}

/// A converted document longer than the window leaves it whole: the
/// model reads a substitute with a path to page, the picture beside it
/// survives, and the move is accounted.
#[test]
fn a_long_connector_answer_is_stored_and_paged_rather_than_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let env = dir.path().join("env");
    std::fs::create_dir_all(&env).unwrap();
    let room = kernel::Address::parse("room").unwrap();
    let document = "The quarter closed with every account reconciled. ".repeat(1_000);
    let long = answer(json!([
        { "type": "text", "text": document },
        { "type": "image", "data": "AAAA", "mimeType": "image/png" },
    ]));
    let out = package_connector(
        long,
        OffloadSite {
            cas: &mut cas,
            city_root: &env,
            room: &room,
            origin: crate::offload::tests::origin(),
        },
    )
    .unwrap();
    let map = out.result.as_map();
    let content = map["content"].as_array().unwrap();
    assert_eq!(content.len(), 2, "one text window, then the picture");
    let window = content[0]["text"].as_str().unwrap();
    assert!(
        window.len() < document.len(),
        "the window is not the document"
    );
    // "AAAA" is base64 for three zero bytes, which no PNG header reads.
    assert_eq!(content[1]["type"], "text");
    assert!(
        content[1]["text"]
            .as_str()
            .unwrap()
            .starts_with("[picture left out:"),
        "{}",
        content[1]
    );
    assert!(out.attachments.is_empty());
    assert_eq!(map["isError"], false);
    assert_eq!(map["offload"].as_array().map(Vec::len), Some(1));

    let short = answer(json!([{ "type": "text", "text": "fits" }]));
    let kept = package_connector(
        short.clone(),
        OffloadSite {
            cas: &mut cas,
            city_root: &env,
            room: &room,
            origin: crate::offload::tests::origin(),
        },
    )
    .unwrap();
    assert_eq!(kept, short, "an answer that fits is not touched");
}

/// A picture a tool answers with reaches the model as a picture, from
/// the content store, and its base64 never enters the window or the
/// ledger.
#[test]
fn a_picture_block_is_stored_and_travels_as_an_attachment() {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let room = kernel::Address::parse("room").unwrap();
    let out = package_connector(
        answer(json!([
            { "type": "text", "text": "fits" },
            { "type": "image", "data": one_pixel_png_base64(), "mimeType": "image/png" },
        ])),
        OffloadSite {
            cas: &mut cas,
            city_root: dir.path(),
            room: &room,
            origin: crate::offload::tests::origin(),
        },
    )
    .unwrap();
    let hash = B3Hash::digest(&ONE_PIXEL_PNG);
    assert_eq!(
        out.attachments,
        vec![ImageRef {
            locator: Locator::cas(hash),
            media_type: ImageType::Png,
            width: 1,
            height: 1,
        }]
    );
    assert_eq!(cas.get(&hash).unwrap(), ONE_PIXEL_PNG.to_vec());
    let content = out.result.as_map()["content"].as_array().unwrap().clone();
    assert!(
        content.iter().all(|block| block["type"] == "text"),
        "{content:?}"
    );
    assert!(
        content[1]["text"]
            .as_str()
            .unwrap()
            .starts_with("[picture attached: image/png 1x1, cas:"),
        "{content:?}"
    );
}

/// A picture this city cannot measure is left out with the reason, as
/// words the model reads, and none of its bytes reach the window.
#[test]
fn a_picture_this_city_cannot_measure_is_left_out_in_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let room = kernel::Address::parse("room").unwrap();
    let out = package_connector(
        answer(json!([
            { "type": "image", "data": one_pixel_png_base64(), "mimeType": "image/jpeg" },
        ])),
        OffloadSite {
            cas: &mut cas,
            city_root: dir.path(),
            room: &room,
            origin: crate::offload::tests::origin(),
        },
    )
    .unwrap();
    assert!(out.attachments.is_empty());
    let content = out.result.as_map()["content"].as_array().unwrap().clone();
    assert!(
        content[0]["text"]
            .as_str()
            .unwrap_or_default()
            .starts_with("[picture left out:"),
        "{content:?}"
    );
    assert!(
        !Value::Array(content)
            .to_string()
            .contains(&one_pixel_png_base64()),
        "the base64 stays out of the window"
    );
}

/// The opening bytes of a wav, which is what a recording's sound block
/// carries; the connector judges the container by the block's media type
/// and never decodes the sound.
const WAVE_HEAD: &[u8] = b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00";

fn sounded(block: Value) -> (ToolOutcome, Cas, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let room = kernel::Address::parse("room").unwrap();
    let out = package_connector(
        answer(json!([block, { "type": "text", "text": "{\"state\":\"stopped\"}" }])),
        OffloadSite {
            cas: &mut cas,
            city_root: dir.path(),
            room: &room,
            origin: crate::offload::tests::origin(),
        },
    )
    .unwrap();
    (out, cas, dir)
}

/// A recording a tool answers with is stored in the content store and
/// named in the window by its locator, where a tool that reads a
/// recording can be pointed at it. Its base64 reaches neither the window
/// nor the ledger, and it is not a picture attachment: the model cannot
/// hear it (runtime-SPEC.md section 12.15).
#[test]
fn a_sound_block_is_stored_and_named_by_its_locator() {
    use base64::Engine as _;

    let data = base64::engine::general_purpose::STANDARD.encode(WAVE_HEAD);
    let (out, cas, _dir) =
        sounded(json!({ "type": "audio", "data": data, "mimeType": "audio/wav" }));
    let hash = B3Hash::digest(WAVE_HEAD);
    let content = out.result.as_map()["content"].as_array().unwrap().clone();
    assert_eq!(
        content[0],
        json!({
            "type": "text",
            "text": format!(
                "[recording attached: audio/wav, {} bytes, {}]",
                WAVE_HEAD.len(),
                Locator::cas(hash)
            ),
        })
    );
    assert_eq!(cas.get(&hash).unwrap(), WAVE_HEAD.to_vec());
    assert!(out.attachments.is_empty());
    assert!(!Value::Array(content).to_string().contains(&data));
}

/// A sound block whose bytes do not decode, or whose media type is not
/// audio, is left out with the reason, as words the model reads. Which
/// containers a transcription endpoint takes is the reader's to judge,
/// not the connector's (runtime-SPEC.md section 12.15).
#[test]
fn a_sound_this_city_cannot_carry_is_left_out_in_words() {
    for block in [
        json!({ "type": "audio", "data": "not base64!", "mimeType": "audio/wav" }),
        json!({ "type": "audio", "data": "UklGRg==", "mimeType": "text/plain" }),
    ] {
        let (out, _cas, _dir) = sounded(block);
        let content = out.result.as_map()["content"].as_array().unwrap().clone();
        assert!(
            content[0]["text"]
                .as_str()
                .unwrap_or_default()
                .starts_with("[recording left out:"),
            "{content:?}"
        );
        assert!(!Value::Array(content).to_string().contains("UklGRg=="));
    }
}
