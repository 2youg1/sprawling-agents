// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote listener: a loopback port of its own, opened with the door
//! and closed with it, that the route makes reachable from outside
//! (`crates/sprawling/spec/Outside/Conduit.lean` §8-139).
//!
//! It answers two WebSocket paths. [`PAIR_PATH`] carries one pairing
//! handshake and ends (crates/remote_access/Spec.lean §8-6). [`SESSION_PATH`]
//! carries a session handshake, then relays: each sealed frame from the
//! device is judged by the conduit and, when the door permits it, sent
//! to the city's own `/ws` on this machine as a client would send it;
//! each frame the city sends comes back sealed. Every message on either
//! path is binary.
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
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use kernel::event::record::RemoteClosing;
use kernel::{AxCode, AxError};
use remote_access::handshake::{Finish, Hello, PairHello};
use remote_access::route::Opened;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

use super::conduit::{Conduit, Step};
use super::console::Lasting;
use super::keeper::{Doorway, Phase};

/// Where a device runs its pairing handshake.
pub(crate) const PAIR_PATH: &str = "/remote/pair";
/// Where a paired device opens a session.
pub(crate) const SESSION_PATH: &str = "/remote/session";

/// How often the listener asks the door whether its time is up.
const TICK: Duration = Duration::from_secs(1);

type Device = WebSocketStream<tokio::net::TcpStream>;

/// What the relay needs to reach the city it serves.
#[derive(Clone)]
pub(crate) struct Reaching {
    /// The runtime the listener's tasks run on.
    pub(crate) runtime: tokio::runtime::Handle,
    /// The city's own listener, as a client on this machine reaches it.
    pub(crate) city: SocketAddr,
    /// The pairing token the city's port asks for, if it asks for one.
    pub(crate) token: Option<String>,
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
    doorway.attend(Box::new(Answering(task.abort_handle())))?;
    Ok(opened)
}

/// The listener's task, ended when the door drops it.
struct Answering(tokio::task::AbortHandle);

impl Drop for Answering {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn answer(listener: tokio::net::TcpListener, doorway: Doorway, reaching: Reaching) {
    let now = tokio::time::Instant::now();
    let mut ticks = tokio::time::interval_at(now.checked_add(TICK).unwrap_or(now), TICK);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                // A connection that fails before it is accepted is that
                // connection's loss; the listener keeps answering.
                if let Ok((stream, _)) = accepted {
                    drop(tokio::spawn(connection(stream, doorway.clone(), reaching.clone())));
                }
            }
            _ = ticks.tick() => {
                let door = doorway.clone();
                match tokio::task::spawn_blocking(move || door.tick()).await {
                    Ok(Ok(Phase::Open)) => {}
                    Ok(Ok(Phase::Closed)) => return,
                    Ok(Err(refused)) => {
                        eprintln!("  the remote door could not close at its time: {refused}");
                        eprintln!("  {}", refused.recovery());
                        return;
                    }
                    Err(_) => return,
                }
            }
        }
    }
}

#[expect(
    clippy::result_large_err,
    reason = "the handshake callback's error type is tungstenite's own response"
)]
async fn connection(stream: tokio::net::TcpStream, doorway: Doorway, reaching: Reaching) {
    let mut asked = String::new();
    let shook =
        tokio_tungstenite::accept_hdr_async(stream, |request: &Request, response: Response| {
            request.uri().path().clone_into(&mut asked);
            Ok(response)
        })
        .await;
    let Ok(mut device) = shook else { return };
    let ended = match asked.as_str() {
        PAIR_PATH => pairing(&mut device, &doorway).await,
        SESSION_PATH => session(&mut device, &doorway, &reaching).await,
        other => Err(AxError::failure(
            AxCode::WireMismatch,
            "answer a remote device",
            format!("no path `{other}`"),
        )
        .with_recovery(format!("connect to {PAIR_PATH} or {SESSION_PATH}"))),
    };
    // The device learns why in the close frame's reason, a stable code
    // and nothing else: a stranger on the route reads it too.
    let reason = ended.err().map_or("", |refused| refused.code().as_str());
    let close = tokio_tungstenite::tungstenite::protocol::CloseFrame {
        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
        reason: reason.into(),
    };
    drop(device.close(Some(close)).await);
}

async fn pairing(device: &mut Device, doorway: &Doorway) -> Result<(), AxError> {
    let hello = PairHello::from_bytes(&next_binary(device).await?)?;
    let (reply, waiting) = doorway.pair_reply(&hello)?;
    send(device, reply.as_bytes().to_vec()).await?;
    let sealed = next_binary(device).await?;
    let door = doorway.clone();
    let answer = blocking(move || door.claim(waiting, &sealed)).await?;
    send(device, answer).await
}

async fn session(
    device: &mut Device,
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
            from_device = device.next() => match from_device {
                Some(Ok(Message::Binary(sealed))) => match conduit.judge(&sealed)? {
                    Step::Forward(text) => city.send(Message::Text(text.into())).await.map_err(|source| lost(&source))?,
                    Step::Answer(sealed) => send(device, sealed).await?,
                    Step::Lock => {
                        let door = doorway.clone();
                        blocking(move || door.close(RemoteClosing::Locked)).await?;
                        return Ok(());
                    }
                },
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Text(_))) => return Err(binary_only()),
                Some(Ok(Message::Close(_)) | Err(_)) | None => return Ok(()),
            },
            from_city = city.next() => match from_city {
                Some(Ok(Message::Text(text))) => send(device, conduit.seal_for_device(text.as_str())?).await?,
                Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Close(_)) | Err(_)) | None => return Ok(()),
            },
        }
    }
}

async fn next_binary(device: &mut Device) -> Result<Vec<u8>, AxError> {
    loop {
        match device.next().await {
            Some(Ok(Message::Binary(bytes))) => return Ok(bytes.to_vec()),
            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
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

async fn send(device: &mut Device, bytes: Vec<u8>) -> Result<(), AxError> {
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

fn lost(source: &tokio_tungstenite::tungstenite::Error) -> AxError {
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
