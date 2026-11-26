// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use axum::body::{Body, Bytes, to_bytes};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kernel::B3Hash;
use tokio::sync::{mpsc, watch};

pub(super) const OUTBOUND: &str = "/remote/frame";

const PIPE_HEADER: &str = "x-sprawling-pipe";
const MAX_PIPES: usize = 64;
const FRAMES_IN_FLIGHT: usize = 1;
const OPENING_FRAME_BYTES: usize = 16 * 1024;
const FRAME_PATIENCE: Duration = Duration::from_secs(10);

type Attached = BTreeMap<String, (Delivery, watch::Sender<()>)>;

#[derive(Clone)]
struct Delivery {
    send: mpsc::Sender<Bytes>,
    limit: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub(super) struct Pipes(Arc<Mutex<Option<Attached>>>);

pub(super) struct Lifetime(Pipes);

pub(super) struct Pipe {
    name: HeaderValue,
    token: String,
    pipes: Pipes,
    first: Option<Bytes>,
    input: mpsc::Receiver<Bytes>,
    output: mpsc::Sender<Bytes>,
    stop: watch::Receiver<()>,
    limit: Arc<AtomicUsize>,
}

impl Pipes {
    pub(super) fn listening() -> (Self, Lifetime) {
        let pipes = Self(Arc::new(Mutex::new(Some(BTreeMap::new()))));
        (pipes.clone(), Lifetime(pipes))
    }

    pub(super) fn start(&self, first: Bytes) -> Result<(Pipe, Response), StatusCode> {
        let mut attached = self.attached();
        let attached = attached.as_mut().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        if attached.len() >= MAX_PIPES {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let mut entropy = [0; 32];
        getrandom::fill(&mut entropy).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let token = B3Hash::digest(&entropy).to_string();
        let name = HeaderValue::from_str(&token).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let (publish, input) = mpsc::channel(FRAMES_IN_FLIGHT);
        let (output, responses) = mpsc::channel(FRAMES_IN_FLIGHT);
        if attached.contains_key(&token) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let (close, stop) = watch::channel(());
        let limit = Arc::new(AtomicUsize::new(OPENING_FRAME_BYTES));
        attached.insert(
            token.clone(),
            (
                Delivery {
                    send: publish,
                    limit: Arc::clone(&limit),
                },
                close,
            ),
        );
        let pipe = Pipe {
            name,
            token,
            pipes: self.clone(),
            first: Some(first),
            input,
            output,
            stop,
            limit,
        };
        let response = pipe.response(responses);
        Ok((pipe, response))
    }

    pub(super) async fn deliver(&self, headers: &HeaderMap, body: Body) -> StatusCode {
        let Some(header) = headers.get(header::AUTHORIZATION) else {
            return StatusCode::FORBIDDEN;
        };
        let Ok(header) = header.to_str() else {
            return StatusCode::FORBIDDEN;
        };
        let Some(token) = header.strip_prefix("Bearer ") else {
            return StatusCode::FORBIDDEN;
        };
        let Some(delivery) = self
            .attached()
            .as_ref()
            .and_then(|attached| attached.get(token))
            .map(|(delivery, _)| delivery.clone())
        else {
            return StatusCode::FORBIDDEN;
        };
        let deadline = tokio::time::Instant::now().checked_add(FRAME_PATIENCE);
        let Some(deadline) = deadline else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        };
        let permit = match tokio::time::timeout_at(deadline, delivery.send.reserve()).await {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) => return StatusCode::FORBIDDEN,
            Err(_) => return StatusCode::REQUEST_TIMEOUT,
        };
        let limit = delivery.limit.load(Ordering::Relaxed);
        let body = match tokio::time::timeout_at(deadline, to_bytes(body, limit)).await {
            Ok(Ok(body)) => body,
            Ok(Err(_)) => return StatusCode::BAD_REQUEST,
            Err(_) => return StatusCode::REQUEST_TIMEOUT,
        };
        permit.send(body);
        StatusCode::NO_CONTENT
    }

    fn attached(&self) -> MutexGuard<'_, Option<Attached>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Lifetime {
    fn drop(&mut self) {
        drop(self.0.attached().take());
    }
}

impl Pipe {
    fn response(&self, input: mpsc::Receiver<Bytes>) -> Response {
        let messages = futures_util::stream::unfold(input, |mut input| async move {
            input
                .recv()
                .await
                .map(|message| (Ok::<Bytes, Infallible>(message), input))
        });
        let mut response = Body::from_stream(messages).into_response();
        let headers = response.headers_mut();
        headers.insert(PIPE_HEADER, self.name.clone());
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
    }

    pub(super) fn authenticated(&self) {
        self.limit.store(wire::DROP_BYTES_MAX, Ordering::Relaxed);
    }

    pub(super) async fn recv(&mut self) -> Option<Bytes> {
        if let Some(first) = self.first.take() {
            return Some(first);
        }
        tokio::select! {
            biased;
            _closed = self.stop.changed() => None,
            () = self.output.closed() => None,
            incoming = self.input.recv() => incoming,
        }
    }

    pub(super) async fn send(&self, bytes: &[u8]) -> bool {
        self.packet(0, bytes).await
    }

    pub(super) async fn close(&self, code: &str) {
        let _sent = self.packet(1, code.as_bytes()).await;
    }

    async fn packet(&self, kind: u8, bytes: &[u8]) -> bool {
        let Ok(length) = u32::try_from(bytes.len()) else {
            return false;
        };
        let mut packet = Vec::with_capacity(5_usize.saturating_add(bytes.len()));
        packet.push(kind);
        packet.extend_from_slice(&length.to_be_bytes());
        packet.extend_from_slice(bytes);
        let mut stop = self.stop.clone();
        tokio::select! {
            biased;
            _closed = stop.changed() => false,
            sent = tokio::time::timeout(FRAME_PATIENCE, self.output.send(Bytes::from(packet))) => {
                matches!(sent, Ok(Ok(())))
            }
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        if let Some(attached) = self.pipes.attached().as_mut() {
            drop(attached.remove(&self.token));
        }
    }
}
