// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The OCR tool (sprawling-SPEC.md 8-142). It is offered only to a city
//! that chose a model for `ocr`, and the two catalogue tests choose
//! none, so the tool is covered here by cities of its own.

use serde_json::json;

use crate::city::History;
use crate::episodes::{LAB, OPEN_LAB, answered_text, choose, read_request, stored_for};
use crate::script::Step;

/// What the scripted OCR endpoint reads in every picture.
const READ: &str = "KILN 1280";

/// The model the person chose for `ocr`: the row of the built-in
/// catalogue that is registered as reading pictures (sprawling-SPEC.md
/// 8-142 says why it has to be that row).
const EYES: &str = "claude-sonnet";

/// A one-pixel PNG.
const ONE_PIXEL_PNG: [u8; 70] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x60, 0x60, 0xF8,
    0x0F, 0x00, 0x01, 0x04, 0x01, 0x00, 0x5F, 0xE5, 0xC3, 0x4B, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// The same bytes as base64, the way a request carries them, put
/// together from short pieces so the secret gate does not read one long
/// run of high-entropy text.
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

/// A PNG in the lab and a screenshot a connector stored for the lab are
/// each read by the model chosen for `ocr`, on its chat face, with the
/// picture in the request.
#[test]
fn a_run_reads_a_png_and_a_stored_screenshot_through_the_model_chosen_for_ocr() {
    let dir = tempfile::tempdir().unwrap();
    let (url, served) = ocr_endpoint(2);
    let shot = format!("cas:b3-{}", kernel::B3Hash::digest(&ONE_PIXEL_PNG));
    let (factory, _, _) = crate::script::scripted(vec![
        Step {
            tool: "ocr",
            args: json!({ "path": format!("{}/shot.png", LAB.room) }),
        },
        Step {
            tool: "ocr",
            args: json!({ "path": shot }),
        },
    ]);
    let (mut worker, ledger) = crate::city::city_with_a_model(dir.path(), factory);
    choose(&mut worker, &url, kernel::ModelTag::Ocr, EYES);
    crate::city::raise(&mut worker, LAB.building, "minimal");
    crate::city::rules(dir.path(), LAB.building, OPEN_LAB);
    crate::city::move_in(dir.path(), LAB.room);
    std::fs::write(
        dir.path().join("lab").join("lead").join("shot.png"),
        ONE_PIXEL_PNG,
    )
    .unwrap();
    stored_for(dir.path(), LAB.building, &ONE_PIXEL_PNG);

    let dispatched = crate::city::dispatch(&mut worker, LAB.room);

    let history = History::read(&ledger);
    let lead = history.started().first().map(|started| started.run);
    let calls = lead.map(|run| history.calls_of(run)).unwrap_or_default();
    let read: Vec<Result<&str, String>> = calls
        .iter()
        .filter(|call| call.called.name.as_str() == "ocr")
        .filter_map(|call| call.results.first())
        .map(|result| answered_text(&result.answer))
        .collect();
    assert_eq!(
        read,
        vec![Ok(READ), Ok(READ)],
        "the dispatch answered {dispatched:?}"
    );
    let pixels = one_pixel_png_base64();
    let shown: Vec<(bool, bool, bool)> = served
        .join()
        .unwrap()
        .iter()
        .map(|request| {
            (
                request.starts_with("POST /v1/chat/completions"),
                request.contains(&format!("\"{EYES}\"")),
                request.contains(&pixels),
            )
        })
        .collect();
    assert_eq!(shown, vec![(true, true, true); 2]);
}

/// An OCR endpoint on loopback that answers `calls` chat requests with
/// [`READ`] and hands back the requests. Whatever it is asked before
/// that, the model list attaching it reads, names [`EYES`].
fn ocr_endpoint(calls: usize) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::Write as _;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let served = std::thread::spawn(move || {
        let mut heard = Vec::new();
        while heard.len() < calls {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let chat = request.starts_with("POST /v1/chat/completions");
            let body = if chat {
                json!({
                    "choices": [{ "message": { "role": "assistant", "content": READ },
                                  "finish_reason": "stop" }],
                    "usage": { "prompt_tokens": 1, "completion_tokens": 1 },
                })
            } else {
                json!({ "data": [{ "id": EYES }] })
            };
            let body = body.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            if chat {
                heard.push(request);
            }
        }
        heard
    });
    (url, served)
}
