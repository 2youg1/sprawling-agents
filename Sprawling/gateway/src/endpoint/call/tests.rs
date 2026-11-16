// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::fakes::{config, fake_provider, request};
use super::super::redemption::{Redemption, redemption, resolver};
use super::*;
use kernel::AxCode;

#[test]
fn a_conversation_keeps_its_id_from_turn_to_turn() {
    let said = |role, text: &str| kernel::ChatMessage {
        role,
        content: vec![kernel::ContentBlock::Text {
            text: text.to_owned(),
        }],
    };
    let mut first = request();
    first
        .chat
        .messages
        .to_mut()
        .push(said(kernel::Role::User, "Task: probe"));
    let mut later = first.clone();
    later
        .chat
        .messages
        .to_mut()
        .push(said(kernel::Role::Assistant, "a later turn"));
    let mut other = request();
    other
        .chat
        .messages
        .to_mut()
        .push(said(kernel::Role::User, "another conversation"));
    let id = conversation_id(&first).unwrap();
    assert_eq!(id.len(), 64, "{id}");
    assert_eq!(conversation_id(&later).unwrap(), id);
    assert_ne!(conversation_id(&other).unwrap(), id);
}

fn seeing(count: u32) -> ModelRequest<'static> {
    let mut req = request();
    let mut content = Vec::new();
    for i in 0..count {
        let hex = format!("{i:02x}").repeat(32);
        content.push(kernel::ContentBlock::Image(kernel::ImageRef {
            locator: kernel::Locator::parse(&format!("cas:b3-{hex}")).unwrap(),
            media_type: kernel::ImageType::Png,
            width: 8,
            height: 8,
        }));
    }
    req.chat.messages.to_mut().push(kernel::ChatMessage {
        role: kernel::Role::User,
        content,
    });
    req
}

fn deepseek_picture(
    bytes: Vec<u8>,
    media_type: kernel::ImageType,
    width: u32,
    height: u32,
) -> (Endpoint, ModelRequest<'static>) {
    let picture = kernel::ImageRef {
        locator: kernel::Locator::cas(kernel::B3Hash::digest(&bytes)),
        media_type,
        width,
        height,
    };
    let mut called = request();
    called.chat.messages.to_mut().push(kernel::ChatMessage {
        role: kernel::Role::User,
        content: vec![ContentBlock::Image(picture)],
    });
    let mut configured = config("https://api.deepseek.com");
    configured.dialect = DialectKind::OpenAi;
    configured.model = "deepseek-flash".to_owned();
    configured.pricing = None;
    let endpoint = Endpoint::new(
        configured,
        Redemption::new(
            resolver(),
            std::sync::Arc::new(move |_at: &kernel::Locator| Ok(bytes.clone())),
        ),
    )
    .unwrap();
    (endpoint, called)
}

fn wire_picture(wire: &Value) -> Vec<u8> {
    use base64::Engine as _;
    let url = wire["messages"][0]["content"][0]["image_url"]["url"]
        .as_str()
        .unwrap();
    let (_, encoded) = url.split_once(',').unwrap();
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap()
}

#[test]
fn a_deepseek_picture_over_the_side_ceiling_is_resized_at_the_request_edge() {
    let mut bytes = Vec::new();
    image::DynamicImage::new_rgb8(1356, 16384)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    let (endpoint, called) = deepseek_picture(bytes, kernel::ImageType::Jpeg, 1356, 16384);

    let wire = endpoint.wire_request(&called).unwrap();
    let sent = wire_picture(&wire);
    let dimensions = image::ImageReader::new(std::io::Cursor::new(sent))
        .with_guessed_format()
        .unwrap()
        .into_dimensions()
        .unwrap();
    assert_eq!(dimensions, (678, 8192));
}

#[test]
fn understated_picture_dimensions_cannot_bypass_the_ceiling() {
    let mut bytes = Vec::new();
    image::DynamicImage::new_rgb8(8, 9000)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let (endpoint, mut called) = deepseek_picture(bytes, kernel::ImageType::Png, 1, 1);
    let ContentBlock::Image(mut second) = called.chat.messages[0].content[0].clone() else {
        panic!("fixture must contain a picture");
    };
    second.width = 8;
    second.height = 9000;
    called.chat.messages.to_mut()[0]
        .content
        .push(ContentBlock::Image(second));
    let wire = endpoint.wire_request(&called).unwrap();
    let sent = image::load_from_memory(&wire_picture(&wire)).unwrap();
    assert_eq!((sent.width(), sent.height()), (7, 8192));
}

#[test]
fn fifteen_repeated_picture_parts_use_the_lower_ceiling() {
    let mut bytes = Vec::new();
    image::DynamicImage::new_rgb8(8, 5000)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let (mut endpoint, mut called) =
        deepseek_picture(bytes.clone(), kernel::ImageType::Png, 8, 5000);
    let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = reads.clone();
    endpoint.redemption = Redemption::new(
        resolver(),
        std::sync::Arc::new(move |_| {
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(bytes.clone())
        }),
    );
    let content = &mut called.chat.messages.to_mut()[0].content;
    let picture = content[0].clone();
    content.resize(14, picture.clone());
    let wire = endpoint.wire_request(&called).unwrap();
    assert_eq!(
        image::load_from_memory(&wire_picture(&wire))
            .unwrap()
            .height(),
        5000
    );
    called.chat.messages.to_mut()[0].content.push(picture);
    let wire = endpoint.wire_request(&called).unwrap();
    assert_eq!(wire["messages"][0]["content"].as_array().unwrap().len(), 15);
    assert_eq!(
        image::load_from_memory(&wire_picture(&wire))
            .unwrap()
            .height(),
        4096
    );
    assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn a_deepseek_picture_within_the_side_ceiling_keeps_its_original_bytes() {
    let mut bytes = Vec::new();
    image::DynamicImage::new_rgb8(8, 8)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    let (endpoint, called) = deepseek_picture(bytes.clone(), kernel::ImageType::Jpeg, 9000, 9000);

    let wire = endpoint.wire_request(&called).unwrap();
    assert_eq!(wire_picture(&wire), bytes);
}

#[test]
fn an_animated_picture_over_the_side_ceiling_is_not_silently_flattened() {
    let mut bytes = Vec::new();
    image::codecs::gif::GifEncoder::new(&mut bytes)
        .encode_frames([
            image::Frame::new(image::RgbaImage::new(4, 16384)),
            image::Frame::new(image::RgbaImage::new(4, 16384)),
        ])
        .unwrap();
    let (endpoint, called) = deepseek_picture(bytes, kernel::ImageType::Gif, 4, 16384);

    let refused = endpoint.wire_request(&called);
    assert!(
        refused.is_err(),
        "an animated picture crossed without a refusal"
    );
    let err = refused.unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("static frame"), "{err}");
}

#[test]
fn a_success_round_trip_settles_and_maps_the_wave() {
    let body = serde_json::json!({
        "content": [
            { "type": "text", "text": "on it" },
            { "type": "tool_use", "id": "tu_9", "name": "exec", "input": {"cmd": "ls"} },
        ],
        "stop_reason": "tool_use",
        "usage": { "input_tokens": 1_000_000, "output_tokens": 0 },
    })
    .to_string();
    let (url, server) = fake_provider(vec![(200, body)], false);
    let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
    let ret = endpoint.call(&request()).unwrap();
    assert_eq!(ret.calls.len(), 1);
    assert_eq!(ret.calls[0].id, "tu_9");
    assert_eq!(ret.billed_usd_micros, Some(UsdMicros::new(3_000_000)));
    assert_eq!(ret.stop, Some(kernel::StopReason::ToolUse));
    let seen = server.join().unwrap();

    assert!(seen[0].contains("x-api-key: sk-test-0123456789"));
    assert!(seen[0].contains("anthropic-version: 2023-06-01"));

    assert!(
        seen[0]
            .to_ascii_lowercase()
            .contains(concat!("user-agent: sprawling/", env!("CARGO_PKG_VERSION"))),
        "{}",
        seen[0]
    );
    assert!(seen[0].contains("\"user_id\":\"city\""));
    assert!(seen[0].contains("\"model\":\"provider-model\""));
}

#[test]
fn provider_status_errors_map_to_e_provider_with_status_only() {
    let (url, server) = fake_provider(vec![(429, "{}".to_owned())], false);
    let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
    let err = endpoint.call(&request()).unwrap_err();
    assert_eq!(*err.code(), AxCode::Provider);
    assert!(err.subject().contains("429"));
    drop(server);
}

#[test]
fn a_cut_body_is_e_provider_never_a_partial_return() {
    let body = serde_json::json!({
        "content": [ { "type": "text", "text": "half" } ],
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 1, "output_tokens": 1 },
    })
    .to_string();
    let (url, server) = fake_provider(vec![(200, body)], true);
    let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
    let err = endpoint.call(&request()).unwrap_err();
    assert_eq!(*err.code(), AxCode::Provider);
    drop(server);
}

#[test]
fn the_fifth_distinct_picture_in_one_request_is_refused_before_any_byte_is_sent() {
    let endpoint = Endpoint::new(config("http://127.0.0.1:1/v1/messages"), redemption()).unwrap();
    let err = endpoint.wire_request(&seeing(5)).unwrap_err();

    assert_eq!(err, IMAGES_PER_TURN.admit(5).unwrap_err());

    assert!(endpoint.wire_request(&seeing(4)).is_ok());
}

#[test]
fn repeated_references_resolve_once_without_changing_their_wire_positions() {
    let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&reads);
    let endpoint = Endpoint::new(
        config("http://127.0.0.1:1/v1/messages"),
        Redemption::new(
            resolver(),
            std::sync::Arc::new(move |_at: &kernel::Locator| {
                counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(b"the-pixels".to_vec())
            }),
        ),
    )
    .unwrap();
    let mut repeated = seeing(1);
    let picture = repeated.chat.messages.to_mut()[0].content[0].clone();
    for _ in 1..5 {
        repeated.chat.messages.to_mut()[0]
            .content
            .push(picture.clone());
    }

    let wire = endpoint.wire_request(&repeated).unwrap();

    assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(wire.to_string().matches("\"type\":\"image\"").count(), 5);
}

#[test]
fn a_picture_larger_than_the_ceiling_is_refused_with_the_ceiling_named() {
    let endpoint = Endpoint::new(
        config("http://127.0.0.1:1/v1/messages"),
        Redemption::new(
            resolver(),
            std::sync::Arc::new(|_at: &kernel::Locator| Ok(vec![0u8; 5_000_000])),
        ),
    )
    .unwrap();
    let request = seeing(1);
    let at = pictures_in(&request.chat)
        .first()
        .map(|picture| picture.locator.clone())
        .expect("the fixture puts one picture in the chat");
    let err = endpoint.wire_request(&request).unwrap_err();
    assert_eq!(err, IMAGE_MAX_BYTES.admit(&at, 5_000_000).unwrap_err());
}

#[test]
fn typed_generation_is_numeric_and_a_raw_override_still_wins() {
    let mut configured = config("http://127.0.0.1:1/v1/messages");
    configured
        .overrides
        .push(("/temperature".to_owned(), serde_json::json!(0.8)));
    let endpoint = Endpoint::new(configured, redemption()).unwrap();
    let mut called = request();
    called.chat.generation = kernel::GenerationSettings {
        temperature: Some("0.2".to_owned()),
        top_p: Some("0.9".to_owned()),
        top_k: Some("40".to_owned()),
    };
    let wire = endpoint.wire_request(&called).unwrap();
    assert_eq!(wire["temperature"], serde_json::json!(0.8));
    assert_eq!(wire["top_p"], serde_json::json!(0.9));
    assert_eq!(wire["top_k"], serde_json::json!(40));
    called.chat.generation.temperature = Some("not-a-number".to_owned());
    assert_eq!(
        *endpoint.wire_request(&called).unwrap_err().code(),
        AxCode::ConfigInvalid
    );
}

#[test]
fn overrides_create_missing_paths_and_refuse_non_objects() {
    let mut root = serde_json::json!({"a": 1});
    apply_override(&mut root, "/b/c", &Value::Bool(true)).unwrap();
    assert_eq!(root["b"]["c"], true);
    let err = apply_override(&mut root, "/a/c", &Value::Bool(true)).unwrap_err();
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
}
