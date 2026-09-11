// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The listening end, and the humble half of it (ARCHITECTURE section
//! 9). Every branch here is a send, a receive, or the end of a session;
//! the judgements it applies are `channels::reception`'s and the bytes
//! it serves are `channels::assets`'.
//!
//! Five jobs and no policy: serve the client bundle, upgrade a
//! WebSocket, accept an upload, take a credential from a caller on this
//! machine, and let an outside editor drive the city.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.

//! Sockets: sessions, assets, uploads.

pub(crate) async fn upgrade(
    State(state): State<Arc<ShellState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| session(socket, state))
}

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kernel::{AxCode, AxError};
use tokio::sync::broadcast;

use crate::assets::AssetReply;
use crate::auth;
use crate::reception::{BindVerdict, SessionState, SessionStep, decide_bind, decide_frame};
use crate::wire::{ClientFrame, ServerFrame};

use super::config::{AcpBody, ServeConfig, ShellState, router};

use super::reply::{Delivered, Reply, refusal_text};
use std::net::SocketAddr;
/// The shell around [`decide_frame`]: it moves bytes and holds no policy.
/// Every judgement here is the pure function's; every branch below is
/// either a send, a receive, or the end of the session.
pub(crate) async fn session(mut socket: WebSocket, state: Arc<ShellState>) {
    let mut phase = SessionState::AwaitingHello;
    let mut events = state.events.subscribe();
    let mut deltas = state.deltas.subscribe();
    // This session's own refusals, which the worker posts into long
    // after the command was accepted. Unbounded because a refusal must
    // not be dropped and because its rate is the rate at which one
    // person makes mistakes, not the rate of the event stream.
    let (refused, mut refusals) = tokio::sync::mpsc::unbounded_channel::<AxError>();
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { return };
                let Message::Text(text) = message else { continue };
                let Ok(frame) = serde_json::from_str::<ClientFrame>(&text) else {
                    let refusal = AxError::failure(
                        AxCode::WireMismatch,
                        "decode a client frame",
                        "the frame does not match this wire",
                    )
                    .with_recovery("reload the page to fetch the client this server was built with");
                    let _ = send(&mut socket, &ServerFrame::Refusal(Box::new(refusal))).await;
                    return;
                };
                match decide_frame(phase, frame, state.token_digest.as_ref(), state.city.as_ref()) {
                    SessionStep::Welcome(welcome) => {
                        phase = SessionState::Live;
                        if send(&mut socket, &ServerFrame::Welcome(*welcome)).await.is_err() {
                            return;
                        }
                    }
                    SessionStep::Deliver(command) => {
                        let back = refused.clone();
                        let reply = Reply::to(move |error| match back.send(error) {
                            Ok(()) => Delivered::ToThePeer,
                            Err(_) => Delivered::PeerGone,
                        });
                        if let Err(error) = (state.commands)(*command, reply)
                            && send(&mut socket, &ServerFrame::Refusal(Box::new(error))).await.is_err() {
                            return;
                        }
                    }
                    SessionStep::Answer(query) => {
                        // Answering reads the disk and takes a lock, and
                        // it ran here, inside the task that owns this
                        // socket. One `RunHistory` therefore held a
                        // tokio worker thread for the whole read: this
                        // peer received no pushed event for as long as
                        // it lasted, and enough people opening a session
                        // at once could occupy every worker the runtime
                        // has. The blocking pool is where a synchronous
                        // read belongs, and this stays true however fast
                        // the read becomes.
                        let answering = Arc::clone(&state.queries);
                        let asked = *query;
                        let outcome =
                            tokio::task::spawn_blocking(move || answering(asked)).await;
                        let frame = match outcome {
                            Ok(Ok(answer)) => ServerFrame::Answer(Box::new(answer)),
                            Ok(Err(error)) => ServerFrame::Refusal(Box::new(error)),
                            // The pool dropped the work, which means the
                            // runtime is going down; say so rather than
                            // leave the page waiting on a frame that
                            // will never come.
                            Err(_) => ServerFrame::Refusal(Box::new(
                                AxError::failure(
                                    AxCode::StorageFatal,
                                    "answer a query",
                                    "the answering task did not finish",
                                )
                                .with_recovery("ask again; if it repeats, restart the server"),
                            )),
                        };
                        if send(&mut socket, &frame).await.is_err() {
                            return;
                        }
                    }
                    SessionStep::Refuse { error, close } => {
                        let _ = send(&mut socket, &ServerFrame::Refusal(error)).await;
                        if close {
                            return;
                        }
                    }
                }
            }
            // A refusal the worker made after this socket had already
            // answered. It reaches the peer that caused it and nobody
            // else, which is why it travels here and not as an event.
            late = refusals.recv() => {
                let Some(error) = late else { return };
                if send(&mut socket, &ServerFrame::Refusal(Box::new(error))).await.is_err() {
                    return;
                }
            }
            event = events.recv() => {
                match event {
                    Ok(record) => {
                        if phase == SessionState::Live
                            && send(&mut socket, &ServerFrame::Event(Box::new(record))).await.is_err() {
                            return;
                        }
                    }
                    // A slow client loses the middle of the stream rather
                    // than holding the writer back; it recovers from the
                    // ledger on reconnect.
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            // Increments, on their own channel. Lag is ignored without a
            // word: a missed increment is a missed frame of an animation,
            // and the settled text arrives as a record either way.
            said = deltas.recv() => {
                match said {
                    Ok(delta) => {
                        if phase == SessionState::Live
                            && send(&mut socket, &ServerFrame::Delta(delta)).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => {}
                }
            }
        }
    }
}

pub(crate) async fn send(socket: &mut WebSocket, frame: &ServerFrame) -> Result<(), ()> {
    let Ok(text) = serde_json::to_string(frame) else {
        return Err(());
    };
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}

pub(crate) async fn serve_index(State(state): State<Arc<ShellState>>) -> Response {
    asset_response(state.client.lookup("index.html"))
}

pub(crate) async fn serve_asset(
    State(state): State<Arc<ShellState>>,
    axum::extract::Path(asset): axum::extract::Path<String>,
) -> Response {
    asset_response(state.client.lookup(&asset))
}

/// The shell around [`ClientAssets::lookup`]: headers on, policy out.
pub(crate) fn asset_response(reply: AssetReply) -> Response {
    match reply {
        AssetReply::Found {
            bytes,
            content_type,
            gzipped: true,
        } => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CONTENT_ENCODING, "gzip"),
            ],
            bytes,
        )
            .into_response(),
        AssetReply::Found {
            bytes,
            content_type,
            gzipped: false,
        } => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type)],
            bytes,
        )
            .into_response(),
        AssetReply::Miss(err) => (StatusCode::NOT_FOUND, refusal_text(&err)).into_response(),
    }
}

/// Large attachments travel over HTTP, not as WebSocket frames: a frame
/// carrying hundreds of megabytes is the wrong shape, and HTTP already
/// answers ranges, resumption and progress.
/// An outside editor's request. The token is judged here and the
/// verdict travels inward; an unauthenticated request still reaches the
/// admission, because what it may learn is that admission's to decide.
pub(crate) async fn accept_acp(State(state): State<Arc<ShellState>>, body: Bytes) -> Response {
    let Ok(request) = serde_json::from_slice::<AcpBody>(&body) else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send {\"token\":..,\"addr\":..,\"task\":..,\"goal\":..}",
        )
            .into_response();
    };
    let authentic = match state.token_digest.as_ref() {
        // A city with no pairing token configured is a city on loopback
        // only; the door is open to whoever is already on this machine,
        // which is the same rule the control surface follows.
        None => true,
        Some(digest) => auth::verify(Some(request.token.as_str()), digest),
    };
    match (state.acp)(request, authentic) {
        Ok(progress) => (StatusCode::ACCEPTED, Json(progress)).into_response(),
        Err(err) => (StatusCode::FORBIDDEN, refusal_text(&err)).into_response(),
    }
}

/// One recording in, one line of text back.
///
/// The media type is read off the request rather than guessed from the
/// bytes: a browser records into whatever container it has, and it is
/// the only party that knows which. A request with none is refused by
/// name, because a container nobody declared cannot be sent on.
pub(crate) async fn accept_recording(
    State(state): State<Arc<ShellState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(media) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send the recording with the content-type it was recorded in",
        )
            .into_response();
    };
    // The parameters a browser appends (`; codecs=opus`) name the codec
    // inside the container, and what the audio wire routes on is the
    // container.
    let media = media.split(';').next().unwrap_or(media).trim().to_owned();
    match (state.transcribe_sink)(body.to_vec(), media) {
        Ok(text) => (StatusCode::OK, text).into_response(),
        Err(err) => (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response(),
    }
}

pub(crate) async fn accept_upload(State(state): State<Arc<ShellState>>, body: Bytes) -> Response {
    match (state.upload_sink)(body.to_vec()) {
        Ok(id) => (StatusCode::CREATED, id.as_str().to_owned()).into_response(),
        Err(err) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("{}: {}", err.action(), err.recovery()),
        )
            .into_response(),
    }
}

/// Binds and serves until the future is dropped.
///
/// Returns the refusal from [`decide_bind`] without touching the network
/// when the configuration is not allowed to listen.
///
/// # Errors
/// Refuses an exposed bind without a pairing token; propagates the bind and
/// accept failures the operating system reports.
pub async fn serve(config: ServeConfig) -> Result<(), AxError> {
    let face = match decide_bind(&config.addr, config.token_digest.is_some()) {
        BindVerdict::Serve(face) => face,
        BindVerdict::Refuse(err) => return Err(err),
    };
    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|source| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "bind the control surface",
                format!("{}: {source}", config.addr),
            )
            .with_recovery("choose a free port, or stop the process already holding it")
        })?;
    let app = router(&config);
    let _ = face;
    // Connect info, because one route's policy is the peer's address:
    // a credential may only be enrolled from this machine.
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await.map_err(|source| {
        AxError::failure(
            AxCode::StorageFatal,
            "serve the control surface",
            source.to_string(),
        )
        .with_recovery("restart the process; the listener is gone")
    })
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::B3Hash;
    /// The two facts the acp door decides before anything inward runs:
    /// a configured token must match, and a city with none is loopback
    /// only, which is the same rule the control surface follows.
    #[test]
    fn the_acp_door_judges_the_token_where_the_token_lives() {
        let digest = B3Hash::digest(b"pair-me-0123456789");
        assert!(auth::verify(Some("pair-me-0123456789"), &digest));
        assert!(!auth::verify(Some("pair-me-9876543210"), &digest));
        assert!(
            !auth::verify(None, &digest),
            "a request carrying no token is not authentic against a configured one"
        );
    }
}
