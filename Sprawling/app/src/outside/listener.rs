// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub(crate) const PAIR_PATH: &str = "/remote/pair";

pub(crate) const SESSION_PATH: &str = "/remote/session";

const TICK: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub(crate) struct Reaching {

    pub(crate) runtime: tokio::runtime::Handle,

    pub(crate) city: SocketAddr,

    pub(crate) token: String,

    pub(crate) page: Arc<wire::ClientAssets>,
}

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
    let origins = wire::ListenerOrigins::routed(local, opened.url.as_str());
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
        .spawn(answer(listener, doorway.clone(), reaching.clone(), origins));
    let served = crate::browser_tool::serve(local);
    doorway.attend(Box::new(Answering(task.abort_handle(), served)))?;
    Ok(opened)
}

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

#[derive(Clone)]
struct Serving {
    doorway: Doorway,
    reaching: Reaching,
}

async fn answer(
    listener: tokio::net::TcpListener,
    doorway: Doorway,
    reaching: Reaching,
    origins: wire::ListenerOrigins,
) {
    let page = match wire::bundle_routes(
        Arc::clone(&reaching.page),
        &wire::PageHeaders::every_listener(),
    ) {
        Ok(page) => wire::entered(page, wire::Arrival::Page, &origins),
        Err(unspelt) => {
            eprintln!(
                "  the remote listener cannot serve the page: {}",
                unspelt.recovery()
            );
            return;
        }
    };
    let sockets = Router::new()
        .route(PAIR_PATH, get(pair_upgrade))
        .route(SESSION_PATH, get(session_upgrade));
    let routes = wire::entered(sockets, wire::Arrival::Socket, &origins)
        .merge(page)
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
