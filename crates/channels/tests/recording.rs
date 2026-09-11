// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The transcription route answers with the line of text, and refuses a
//! request that does not say what it recorded into.
//!
//! Driven in process through `tower::ServiceExt::oneshot`, for the same
//! reason the enrolment route is: this is a white-box check of the
//! router, and a check that raised the product's server and wrote HTTP
//! by hand would be standing outside, which is `adversary/`'s ground
//! (`xtask boundary`).

#![cfg(feature = "server")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::Request;
use channels::{Answer, Command, Reply, ServeConfig};
use kernel::{AxCode, AxError};
use tower::ServiceExt;

/// What the city behind the route does with a recording.
enum Hearing {
    /// An endpoint is attached and it answered.
    Answers,
    /// No model was chosen to transcribe.
    Unattached,
}

async fn send(hearing: Hearing, media: Option<&str>, bytes: &[u8]) -> (u16, String) {
    let (events, _held) = tokio::sync::broadcast::channel(16);
    let config = ServeConfig {
        deltas: tokio::sync::broadcast::channel(16).0,
        addr: "127.0.0.1:0".parse().unwrap(),
        token_digest: None,
        client: Arc::new(channels::ClientAssets::Embedded(&[])),
        commands: Arc::new(|_, _| Ok(())),
        transcribe_sink: Arc::new(move |body: Vec<u8>, kind: String| match hearing {
            Hearing::Answers => Ok(format!("{} bytes of {kind}", body.len())),
            Hearing::Unattached => Err(AxError::failure(
                AxCode::ToolUnavailable,
                "transcribe a recording",
                "this city has no transcription endpoint",
            )
            .with_recovery("choose a model for `transcribe` in the settings")),
        }),
        events,
        queries: Arc::new(|_| {
            Ok(Answer::Unavailable {
                query: "none".to_owned(),
            })
        }),
        secrets: Arc::new(|_: Command<kernel::Sealed<String>>, _: Reply| Ok(())),
        acp: Arc::new(|_, _| {
            Ok(channels::AcpProgress {
                run: String::new(),
                turns: 0,
                finished: true,
            })
        }),
        upload_sink: Arc::new(|_| {
            Err(AxError::failure(
                AxCode::InvalidArgs,
                "stage an attachment",
                "not in this test",
            ))
        }),
        city: None,
    };
    let peer: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    let app = channels::router(&config).layer(MockConnectInfo(peer));
    let mut request = Request::builder().method("POST").uri("/transcribe");
    if let Some(kind) = media {
        request = request.header("content-type", kind);
    }
    let response = app
        .oneshot(request.body(Body::from(bytes.to_vec())).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// The text comes back to the request that carried the recording. It
/// cannot come back any other way: a command is answered later through
/// the event stream, and what somebody just said has to reach the tab
/// that recorded it.
#[tokio::test]
async fn the_line_of_text_answers_the_request_that_carried_the_recording() {
    let (status, said) = send(Hearing::Answers, Some("audio/webm; codecs=opus"), b"...").await;
    assert_eq!(status, 200, "{said}");
    assert_eq!(
        said, "3 bytes of audio/webm",
        "the container reaches the city without the codec parameter"
    );
}

/// A request that does not say what it recorded into is refused here,
/// because a container nobody declared has no media type to send on.
#[tokio::test]
async fn a_recording_that_does_not_say_its_container_is_refused() {
    let (status, said) = send(Hearing::Answers, None, b"...").await;
    assert_eq!(status, 422, "{said}");
    assert!(said.contains("content-type"), "{said}");
}

/// A city with nothing chosen to transcribe says so with a way out,
/// rather than answering an empty line that reads as silence.
#[tokio::test]
async fn a_city_with_no_transcription_endpoint_says_what_to_do() {
    let (status, said) = send(Hearing::Unattached, Some("audio/webm"), b"...").await;
    assert_eq!(status, 422, "{said}");
    assert!(said.contains("transcribe"), "{said}");
}
