// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote listener: a loopback port of its own, opened with the door
//! and closed with it, that the route makes reachable from outside
//! (`crates/sprawling/spec/Outside/Conduit.lean` §8-139).
//!
//! It answers two WebSocket paths and the page. [`PAIR_PATH`] carries one
//! pairing handshake and ends (crates/remote_access/Spec.lean §8-6).
//! [`SESSION_PATH`] carries a session handshake, then relays: each sealed
//! frame from the device is judged by the conduit and, when the door
//! permits it, sent to the city's own `/ws` on this machine as a client
//! would send it; each frame the city sends comes back sealed. Every
//! message on either path is binary. Every other `GET` is answered with
//! the client bundle through `wire::bundle_routes`, the same table the
//! city's own port serves it with, so the address in the pairing code
//! opens the page that pairs (crates/remote_access/Spec.lean §8-10).
//!
//! The listener is its own port rather than a route on the city's port
//! because the route makes it reachable from outside while the city's
//! port stays where `serve` bound it, guarded by its own token. Nothing
//! but this relay reaches the city from here.
//!
//! The door keeper's work that writes a line or a file runs on the
//! blocking pool: the ledger relay waits for the city's one writer, and
//! a socket task that waited with it would hold a reactor thread.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use kernel::event::record::RemoteClosing;
use kernel::{AxCode, AxError};
use remote_access::handshake::{Finish, Hello, PairHello};
use remote_access::route::Opened;
use tokio_tungstenite::tungstenite::Message as CityMessage;

use super::conduit::{Conduit, Step};
use super::console::Lasting;
use super::keeper::{Doorway, Phase};

/// Where a device runs its pairing handshake.
pub(crate) const PAIR_PATH: &str = "/remote/pair";
/// Where a paired device opens a session.
pub(crate) const SESSION_PATH: &str = "/remote/session";

/// How often the listener asks the door whether its time is up.
const TICK: Duration = Duration::from_secs(1);

/// What the relay needs to reach the city it serves.
#[derive(Clone)]
pub(crate) struct Reaching {
    /// The runtime the listener's tasks run on.
    pub(crate) runtime: tokio::runtime::Handle,
    /// The city's own listener, as a client on this machine reaches it.
    pub(crate) city: SocketAddr,
    /// The pairing token the city's port asks for, if it asks for one.
    pub(crate) token: Option<String>,
    /// The client bundle the city's port serves, served here too.
    pub(crate) page: Arc<wire::ClientAssets>,
}

/// Binds the remote listener, opens the door through the route, then
/// answers until the door closes.
///
/// # Errors
/// No loopback port; every refusal of `Doorway::open`.
pub(super) fn open(
    doorway: &Doorway,
    reaching: &Reaching,
    lasting: Lasting,
) -> Result<Opened, AxError> {
    let socket = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .and_then(|socket| socket.set_nonblocking(true).map(|()| socket))
        .map_err(|source| unbound(&source))?;
    let local = socket.local_addr().map_err(|source| unbound(&source))?;
    let opened = doorway.open(local, lasting)?;
    let listener = {
        let _entered = reaching.runtime.enter();
        tokio::net::TcpListener::from_std(socket).map_err(|source| unbound(&source))
    };
    let listener = match listener {
        Ok(listener) => listener,
        Err(refused) => {
            doorway.close(RemoteClosing::Console)?;
            return Err(refused);
        }
    };
    let task = reaching
        .runtime
        .spawn(answer(listener, doorway.clone(), reaching.clone()));
    let served = crate::browser_tool::serve(local);
    doorway.attend(Box::new(Answering(task.abort_handle(), served)))?;
    Ok(opened)
}

/// The listener's task, ended when the door drops it, and its place among
/// the addresses the browser tools refuse (sprawling D74).
struct Answering(
    tokio::task::AbortHandle,
    #[expect(
        dead_code,
        reason = "held for its drop, which takes the address back when the door drops the listener"
    )]
    crate::browser_tool::Served,
);

impl Drop for Answering {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// What every route of the listener reads.
#[derive(Clone)]
struct Serving {
    doorway: Doorway,
    reaching: Reaching,
}

/// Serves the two paths and the page until the door says its time is up.
async fn answer(listener: tokio::net::TcpListener, doorway: Doorway, reaching: Reaching) {
    let routes = Router::new()
        .route(PAIR_PATH, get(pair_upgrade))
        .route(SESSION_PATH, get(session_upgrade))
        .merge(wire::bundle_routes(Arc::clone(&reaching.page)))
        .with_state(Serving {
            doorway: doorway.clone(),
            reaching,
        });
    tokio::select! {
        served = axum::serve(listener, routes) => {
            if let Err(lost) = served {
                eprintln!("  the remote listener stopped answering: {lost}");
            }
        }
        () = keep_time(doorway) => {}
    }
}

/// Asks the door each [`TICK`] whether its time is up, and returns once
/// it is closed.
async fn keep_time(doorway: Doorway) {
    let now = tokio::time::Instant::now();
    let mut ticks = tokio::time::interval_at(now.checked_add(TICK).unwrap_or(now), TICK);
    loop {
        ticks.tick().await;
        let door = doorway.clone();
        match tokio::task::spawn_blocking(move || door.tick()).await {
            Ok(Ok(Phase::Open)) => {}
            Ok(Ok(Phase::Closed)) | Err(_) => return,
            Ok(Err(refused)) => {
                eprintln!("  the remote door could not close at its time: {refused}");
                eprintln!("  {}", refused.recovery());
                return;
            }
        }
    }
}

async fn pair_upgrade(State(serving): State<Serving>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |mut device| async move {
        let ended = pairing(&mut device, &serving.doorway).await;
        close(device, ended).await;
    })
}

async fn session_upgrade(State(serving): State<Serving>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |mut device| async move {
        let ended = session(&mut device, &serving.doorway, &serving.reaching).await;
        close(device, ended).await;
    })
}

/// Ends a connection. The device learns why in the close frame's
/// reason, a stable code and nothing else: a stranger on the route
/// reads it too.
async fn close(mut device: WebSocket, ended: Result<(), AxError>) {
    let reason = ended
        .err()
        .map_or_else(String::new, |refused| refused.code().as_str().to_owned());
    let frame = CloseFrame {
        code: close_code::NORMAL,
        reason: reason.into(),
    };
    drop(device.send(Message::Close(Some(frame))).await);
}

async fn pairing(device: &mut WebSocket, doorway: &Doorway) -> Result<(), AxError> {
    let hello = PairHello::from_bytes(&next_binary(device).await?)?;
    let (reply, waiting) = doorway.pair_reply(&hello)?;
    send(device, reply.as_bytes().to_vec()).await?;
    let sealed = next_binary(device).await?;
    let door = doorway.clone();
    let answer = blocking(move || door.claim(waiting, &sealed)).await?;
    send(device, answer).await
}

async fn session(
    device: &mut WebSocket,
    doorway: &Doorway,
    reaching: &Reaching,
) -> Result<(), AxError> {
    let hello = Hello::from_bytes(&next_binary(device).await?)?;
    let (reply, waiting) = doorway.session_reply(&hello)?;
    send(device, reply.as_bytes().to_vec()).await?;
    let finish = Finish::from_bytes(&next_binary(device).await?)?;
    let door = doorway.clone();
    let admitted = blocking(move || door.admit(waiting, &finish)).await?;
    let (mut city, _) = tokio_tungstenite::connect_async(format!("ws://{}/ws", reaching.city))
        .await
        .map_err(|source| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "reach the city for a remote device",
                source.to_string(),
            )
            .with_recovery("the city's own listener did not answer; restart the city")
        })?;
    let mut conduit = Conduit::new(doorway.clone(), admitted, reaching.token.clone());
    loop {
        tokio::select! {
            from_device = device.recv() => match from_device {
                Some(Ok(Message::Binary(sealed))) => match conduit.judge(&sealed)? {
                    Step::Forward(text) => city.send(CityMessage::Text(text.into())).await.map_err(|source| lost(&source))?,
                    Step::Answer(sealed) => send(device, sealed).await?,
                    Step::Lock => {
                        let door = doorway.clone();
                        blocking(move || door.close(RemoteClosing::Locked)).await?;
                        return Ok(());
                    }
                },
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                Some(Ok(Message::Text(_))) => return Err(binary_only()),
                Some(Ok(Message::Close(_)) | Err(_)) | None => return Ok(()),
            },
            from_city = city.next() => match from_city {
                Some(Ok(CityMessage::Text(text))) => send(device, conduit.seal_for_device(text.as_str())?).await?,
                Some(Ok(CityMessage::Binary(_) | CityMessage::Ping(_) | CityMessage::Pong(_) | CityMessage::Frame(_))) => {}
                Some(Ok(CityMessage::Close(_)) | Err(_)) | None => return Ok(()),
            },
        }
    }
}

async fn next_binary(device: &mut WebSocket) -> Result<Vec<u8>, AxError> {
    loop {
        match device.recv().await {
            Some(Ok(Message::Binary(bytes))) => return Ok(bytes.to_vec()),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            Some(Ok(Message::Text(_))) => return Err(binary_only()),
            Some(Ok(Message::Close(_)) | Err(_)) | None => {
                return Err(AxError::failure(
                    AxCode::WireMismatch,
                    "hear a remote device",
                    "the device left before the handshake ended",
                )
                .with_recovery("connect again"));
            }
        }
    }
}

async fn send(device: &mut WebSocket, bytes: Vec<u8>) -> Result<(), AxError> {
    device
        .send(Message::Binary(bytes.into()))
        .await
        .map_err(|source| lost(&source))
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AxError> + Send + 'static,
) -> Result<T, AxError> {
    tokio::task::spawn_blocking(work).await.map_err(|joined| {
        AxError::failure(
            AxCode::StorageFatal,
            "keep the remote door",
            joined.to_string(),
        )
        .with_recovery("restart the city: the door starts closed")
    })?
}

fn binary_only() -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "hear a remote device",
        "a text message where only sealed bytes travel",
    )
    .with_recovery("reload the page: it speaks another version of the remote door")
}

fn lost(source: &dyn std::fmt::Display) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "relay a remote session",
        source.to_string(),
    )
    .with_recovery("connect again")
}

fn unbound(source: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "bind the remote listener",
        source.to_string(),
    )
    .with_recovery("this machine refused a loopback port; close what holds them and try again")
}
