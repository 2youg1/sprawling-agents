// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use super::*;
use kernel::Payload;
use memory::Cas;
use serde_json::{Value, json};

fn answer(content: Value) -> ToolOutcome {
    let Value::Object(map) = json!({ "content": content, "isError": false }) else {
        unreachable!("a literal object")
    };
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
    let picture = json!({ "type": "image", "data": "AAAA", "mimeType": "image/png" });
    let long = answer(json!([
        { "type": "text", "text": document },
        picture,
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
    assert_eq!(content[1], picture);
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
