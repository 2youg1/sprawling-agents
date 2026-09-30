// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The drop route hands the file's name and bytes to the city and
//! answers with the path the city kept it at (wire-SPEC.md 8-49).
//!
//! Driven in process through `tower::ServiceExt::oneshot`, for the same
//! reason the recording route is: this is a white-box check of the
//! router, and a check that raised the product's server would be
//! standing outside, which is `adversary/`'s ground (`xtask boundary`).

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
use kernel::{AxCode, AxError};
use tower::ServiceExt;
use wire::{Answer, Command, Reply, ServeConfig};

async fn send(uri: &str, bytes: Vec<u8>) -> (u16, String) {
    let (events, _held) = tokio::sync::broadcast::channel(16);
    let config = ServeConfig {
        deltas: tokio::sync::broadcast::channel(16).0,
        logs: tokio::sync::broadcast::channel(16).0,
        outputs: tokio::sync::broadcast::channel(16).0,
        outputs_so_far: Arc::new(Vec::new),
        monitor: wire::MonitorFeed {
            watch: Arc::new(|_| -> Box<dyn Send> { Box::new(()) }),
            samples: tokio::sync::broadcast::channel(1).0,
        },
        client: Arc::new(wire::ClientAssets::Embedded(&[])),
        commands: Arc::new(|_, _| Ok(())),
        transcribe_sink: Arc::new(|_, _| Ok(String::new())),
        drop_sink: Arc::new(|name: &str, bytes: &[u8]| {
            if name.contains('/') {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "keep a dropped file",
                    format!("{name}: not one file name"),
                )
                .with_recovery("rename the file and drop it again"));
            }
            Ok(format!("/city/dropped/{}/{name}", bytes.len()))
        }),
        events,
        queries: Arc::new(|_| {
            (
                kernel::Seq::FIRST,
                Ok(Answer::Unavailable {
                    query: "none".to_owned(),
                }),
            )
        }),
        secrets: Arc::new(|_: Command<kernel::Sealed<String>>, _: Reply| Ok(())),
        acp: Arc::new(|_, _| {
            Ok(wire::AcpProgress {
                run: String::new(),
                turns: 0,
                finished: true,
            })
        }),
        city: None,
        head: Arc::default(),
        epoch: None,
    };
    let wire::BindVerdict::Serve(face) = wire::decide_bind(&"127.0.0.1:0".parse().unwrap(), None)
    else {
        panic!("this test serves a loopback address");
    };
    let peer: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    let app = wire::router(&config, face).layer(MockConnectInfo(peer));
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .body(Body::from(bytes))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// The name arrives percent-encoded and reaches the city as the person
/// spelled it, and the path the city answers is the whole response.
#[tokio::test]
async fn the_kept_path_answers_the_request_that_carried_the_file() {
    let (status, said) = send("/drop?name=%E7%AC%94%E8%AE%B0.txt", b"four".to_vec()).await;
    assert_eq!(
        (status, said.as_str()),
        (200, "/city/dropped/4/\u{7b14}\u{8bb0}.txt")
    );
}

/// A request that names no file is refused before the city is asked.
#[tokio::test]
async fn a_drop_without_a_name_is_refused() {
    let (status, said) = send("/drop", b"four".to_vec()).await;
    assert_eq!(status, 422, "{said}");
    assert!(said.contains("name"), "{said}");
}

/// The city's refusal comes back with its recovery, not as silence.
#[tokio::test]
async fn the_citys_refusal_comes_back_whole() {
    let (status, said) = send("/drop?name=a%2Fb", b"four".to_vec()).await;
    assert_eq!(status, 422, "{said}");
    assert!(said.contains("rename the file"), "{said}");
}

/// A file larger than axum's own 2 MiB default still arrives: the limit
/// on this route is the one the SPEC states.
#[tokio::test]
async fn a_file_past_the_framework_default_still_arrives() {
    let (status, said) = send("/drop?name=big.bin", vec![0; 3 * 1024 * 1024]).await;
    assert_eq!(
        (status, said.as_str()),
        (200, "/city/dropped/3145728/big.bin")
    );
}
