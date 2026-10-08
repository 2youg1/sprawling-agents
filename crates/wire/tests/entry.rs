// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The regression the local door was rebuilt for: a page on another
//! site, or a request naming another host, reached the city's socket and
//! its acting doors on loopback without presenting anything
//! (`crates/wire/spec/Reception/Entry.lean` §8-94).
//!
//! Driven in process through `tower::ServiceExt::oneshot`, as the other
//! route tests are: this judges the router's own table, and a test that
//! raised the product's server and spoke HTTP from outside would be
//! `tools/adversary/`'s ground (`xtask boundary`).

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
use tower::ServiceExt;
use wire::{Answer, Command, Reply, ServeConfig};

const AT: &str = "127.0.0.1:8787";

fn config() -> ServeConfig {
    ServeConfig {
        deltas: tokio::sync::broadcast::channel(16).0,
        logs: tokio::sync::broadcast::channel(16).0,
        outputs: tokio::sync::broadcast::channel(16).0,
        outputs_so_far: Arc::new(Vec::new),
        monitor: wire::MonitorFeed {
            watch: Arc::new(|_| -> Box<dyn Send> { Box::new(()) }),
            samples: tokio::sync::broadcast::channel(1).0,
            beat: Arc::new(|_| {}),
        },
        client: Arc::new(wire::ClientAssets::Embedded(&[])),
        commands: Arc::new(|_, _| Ok(())),
        drop_sink: Arc::new(|_, _| Ok(String::new())),
        transcribe_sink: Arc::new(|_, _| Ok(String::new())),
        events: tokio::sync::broadcast::channel(16).0,
        queries: Arc::new(|_| {
            (
                kernel::Seq::FIRST,
                Ok(Answer::Unavailable {
                    query: "none".to_owned(),
                    reason: None,
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
    }
}

/// One request at the router of a city listening on [`AT`], with the
/// headers a browser or a native client would send, answered with its
/// status.
async fn status(method: &str, path: &str, headers: &[(&str, &str)]) -> u16 {
    let at: SocketAddr = AT.parse().unwrap();
    let wire::BindVerdict::Serve(face) =
        wire::decide_bind(&at, Some(kernel::B3Hash::digest(b"native-key")))
    else {
        panic!("a loopback address with a key is served");
    };
    let app = wire::router(&config(), face).layer(MockConnectInfo(at));
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    app.oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
        .as_u16()
}

/// What a browser sends to open the socket, less the Origin and Host
/// each case chooses.
const UPGRADE: [(&str, &str); 4] = [
    ("connection", "upgrade"),
    ("upgrade", "websocket"),
    ("sec-websocket-version", "13"),
    ("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="),
];

#[tokio::test]
async fn a_cross_site_page_is_refused_before_the_socket_upgrades() {
    for origin in [
        "http://evil.example",
        "http://127.0.0.1:5173",
        "http://localhost:3000",
        "null",
    ] {
        let mut headers = UPGRADE.to_vec();
        headers.extend([("host", AT), ("origin", origin)]);
        assert_eq!(status("GET", "/ws", &headers).await, 403, "{origin}");
    }
}

#[tokio::test]
async fn a_foreign_host_is_refused_before_the_socket_upgrades_or_the_page_is_served() {
    for host in ["evil.example:8787", "0.0.0.0:8787", "127.0.0.1:9999"] {
        let mut headers = UPGRADE.to_vec();
        headers.push(("host", host));
        assert_eq!(status("GET", "/ws", &headers).await, 403, "{host}");
        assert_eq!(status("GET", "/", &[("host", host)]).await, 403, "{host}");
    }
}

#[tokio::test]
async fn an_acting_door_refuses_a_page_on_another_port_of_this_machine() {
    let headers = [
        ("host", AT),
        ("origin", "http://127.0.0.1:5173"),
        ("sec-fetch-site", "same-site"),
    ];
    for path in ["/transcribe", "/drop?name=a", "/enroll", "/acp"] {
        assert_eq!(status("POST", path, &headers).await, 403, "{path}");
    }
}

#[tokio::test]
async fn a_preflight_is_refused() {
    let headers = [
        ("host", AT),
        ("origin", "http://evil.example"),
        ("access-control-request-method", "POST"),
    ];
    assert_eq!(status("OPTIONS", "/enroll", &headers).await, 403);
}
