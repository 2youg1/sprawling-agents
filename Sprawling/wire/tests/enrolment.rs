// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code: this stand-in drops what a production reader               would act on, because the assertion is about the status               the route answers and not about the reply reaching anyone"
)]
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
use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs};
use tower::ServiceExt;
use wire::{Answer, AxCode, AxError, Command, Reply, ServeConfig};

enum Worker {
    Stores,

    Refuses,

    Silent,
}

fn captured(reference: &str) -> EventRecord {
    let mut map = serde_json::Map::new();
    map.insert(
        "ref".to_owned(),
        serde_json::Value::String(reference.to_owned()),
    );
    EventRecord::from_draft(
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(1),
            who: "owner".to_owned(),
            addr: None,
            kind: EventKind::SecretCaptured,
            data: Payload::new(map).unwrap(),
            ig: false,
        },
        Seq::FIRST,
        GENESIS_PREV,
    )
}

async fn ask(worker: Worker, body: &str) -> (u16, String) {
    let (events, _held) = tokio::sync::broadcast::channel(16);
    let answering = events.clone();
    let config = ServeConfig {
        workspace: None,
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
        transcribe_sink: Arc::new(|_, _| {
            Err(AxError::failure(
                AxCode::ToolUnavailable,
                "transcribe a recording",
                "this test city has no transcription endpoint",
            )
            .with_recovery("attach an endpoint that offers transcription"))
        }),
        events: events.clone(),
        queries: Arc::new(|_| {
            (
                kernel::Seq::FIRST,
                Ok(Answer::Unavailable {
                    query: "none".to_owned(),
                    reason: None,
                }),
            )
        }),
        secrets: Arc::new(
            move |command: Command<kernel::Sealed<String>>, reply: Reply| {
                let Command::PutSecret { realm, name, .. } = command else {
                    return Ok(());
                };

                let place = kernel::SecretRef::new(&realm, &name)
                    .expect("the body states a realm and a name in the grammar");
                let reference = place.to_string();
                match worker_of(&worker) {
                    Worker::Stores => {
                        let _ = answering.send(wire::Committed::new(captured(&reference)).unwrap());
                    }
                    Worker::Refuses => {
                        let _ = reply.refuse(
                            AxError::failure(
                                AxCode::CredentialMissing,
                                "store a credential",
                                reference,
                            )
                            .with_recovery("this machine has no credential service"),
                        );
                    }
                    Worker::Silent => {}
                }
                Ok(())
            },
        ),
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
        door: wire::LocalDoor::new(
            Vec::new(),
            wire::DoorSenses {
                clock: Arc::new(|| Ok(kernel::TimeMs::new(0))),
                monotonic: Arc::new(|| tokio::time::Instant::now().into_std()),
                entropy: Arc::new(|bytes: &mut [u8]| {
                    bytes.fill(7);
                    Ok(())
                }),
            },
            Arc::new(|_: &[wire::PairedBrowser]| Ok(())),
        )
        .unwrap(),
    };

    let wire::BindVerdict::Serve(face) = wire::decide_bind(
        &"127.0.0.1:2501".parse().unwrap(),
        Some(kernel::B3Hash::digest(b"native-key")),
    ) else {
        panic!("this test serves a loopback address");
    };
    let peer: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    let origins = wire::ListenerOrigins::of("127.0.0.1:2501".parse().unwrap());
    let app = wire::router(&config, face, &origins)
        .unwrap()
        .layer(MockConnectInfo(peer));
    let request = Request::builder()
        .method("POST")
        .uri("/enroll")
        .header("host", "127.0.0.1:2501")
        .header("authorization", "Bearer native-key")
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn worker_of(worker: &Worker) -> Worker {
    match worker {
        Worker::Stores => Worker::Stores,
        Worker::Refuses => Worker::Refuses,
        Worker::Silent => Worker::Silent,
    }
}

const BODY: &str = r#"{"realm":"house","name":"key","value":"sk-not-a-real-key"}"#;

#[tokio::test]
async fn a_stored_credential_answers_with_the_reference_that_replaced_it() {
    let (status, said) = ask(Worker::Stores, BODY).await;

    assert_eq!((status, said.as_str()), (201, "secret:house/key"));
    assert!(
        !said.contains("sk-not-a-real-key"),
        "the value must never come back out"
    );
}

#[tokio::test]
async fn a_refusal_reaches_the_request_that_carried_the_credential() {
    let (status, said) = ask(Worker::Refuses, BODY).await;
    assert_eq!(status, 422, "{said}");
    assert!(said.contains("credential service"), "{said}");
}

#[tokio::test]
async fn neither_answer_within_the_wait_is_its_own_answer() {
    let (status, said) = ask(Worker::Silent, BODY).await;
    assert_eq!(status, 202, "{said}");
    assert!(
        said.contains("has not answered"),
        "202 has to say why it is 202: {said}"
    );
}
