// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use futures_util::{SinkExt, StreamExt};
use kernel::event::record::RemoteClosing;
use kernel::{AxCode, AxError};
use remote_access::handshake::{Finish, HELLO_BYTES, Hello, PAIR_HELLO_BYTES, PairHello};
use remote_access::route::Opened;
use tokio_tungstenite::tungstenite::Message as CityMessage;

use super::conduit::{Conduit, Step};
use super::console::Lasting;
use super::keeper::{Doorway, Phase};
use super::pipe::{self, Pipe, Pipes};

pub(crate) const PAIR_PATH: &str = "/remote/pair";

pub(crate) const SESSION_PATH: &str = "/remote/session";

const TICK: Duration = Duration::from_secs(1);

const HANDSHAKE_PATIENCE: Duration = Duration::from_secs(10);

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
    pipes: Pipes,
}

#[derive(Clone, Copy)]
enum Run {
    Pair,
    Session,
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
        .route(
            PAIR_PATH,
            post(pair_http).layer(DefaultBodyLimit::max(PAIR_HELLO_BYTES)),
        )
        .route(
            SESSION_PATH,
            post(session_http).layer(DefaultBodyLimit::max(HELLO_BYTES)),
        )
        .route(pipe::OUTBOUND, post(frame_http));
    let (pipes, _lifetime) = Pipes::listening();
    let routes = wire::entered(sockets, wire::Arrival::Socket, &origins)
        .merge(page)
        .with_state(Serving {
            doorway: doorway.clone(),
            reaching,
            pipes,
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

async fn pair_http(State(serving): State<Serving>, first: Bytes) -> Response {
    begin_http(serving, first, PAIR_HELLO_BYTES, Run::Pair)
}

async fn session_http(State(serving): State<Serving>, first: Bytes) -> Response {
    begin_http(serving, first, HELLO_BYTES, Run::Session)
}

fn begin_http(serving: Serving, first: Bytes, size: usize, run: Run) -> Response {
    if first.len() != size {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            [(header::CACHE_CONTROL, "no-store")],
        )
            .into_response();
    }
    let (pipe, response) = match serving.pipes.start(first) {
        Ok(opened) => opened,
        Err(status) => return (status, [(header::CACHE_CONTROL, "no-store")]).into_response(),
    };
    let runtime = serving.reaching.runtime.clone();
    drop(runtime.spawn(carry(pipe, serving, run)));
    response
}

async fn frame_http(State(serving): State<Serving>, headers: HeaderMap, body: Body) -> Response {
    (
        serving.pipes.deliver(&headers, body).await,
        [(header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}

async fn carry(mut device: Pipe, serving: Serving, run: Run) {
    let ended = match run {
        Run::Pair => pairing(&mut device, &serving.doorway).await,
        Run::Session => session(&mut device, &serving.doorway, &serving.reaching).await,
    };
    let code = ended
        .as_ref()
        .err()
        .map_or("", |refused| refused.code().as_str());
    device.close(code).await;
}

async fn handshake(device: &mut Pipe) -> Result<Bytes, AxError> {
    match tokio::time::timeout(HANDSHAKE_PATIENCE, device.recv()).await {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) | Err(_) => Err(lost(
            &"the device left or did not finish its remote handshake",
        )),
    }
}

async fn send(device: &Pipe, bytes: &[u8]) -> Result<(), AxError> {
    if device.send(bytes).await {
        Ok(())
    } else {
        Err(lost(&"the browser stopped reading the remote stream"))
    }
}

async fn pairing(device: &mut Pipe, doorway: &Doorway) -> Result<(), AxError> {
    let hello = PairHello::from_bytes(&handshake(device).await?)?;
    let (reply, waiting) = doorway.pair_reply(&hello)?;
    send(device, reply.as_bytes()).await?;
    let sealed = handshake(device).await?;
    let door = doorway.clone();
    let answer = blocking(move || door.claim(waiting, &sealed)).await?;
    send(device, &answer).await
}

async fn session(device: &mut Pipe, doorway: &Doorway, reaching: &Reaching) -> Result<(), AxError> {
    let hello = Hello::from_bytes(&handshake(device).await?)?;
    let (reply, waiting) = doorway.session_reply(&hello)?;
    send(device, reply.as_bytes()).await?;
    let finish = Finish::from_bytes(&handshake(device).await?)?;
    let door = doorway.clone();
    let admitted = blocking(move || door.admit(waiting, &finish)).await?;
    device.authenticated();
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
                Some(sealed) => match conduit.judge(&sealed)? {
                    Step::Forward(text) => city.send(CityMessage::Text(text.into())).await.map_err(|source| lost(&source))?,
                    Step::Answer(sealed) => send(device, &sealed).await?,
                    Step::Lock => {
                        let door = doorway.clone();
                        blocking(move || door.close(RemoteClosing::Locked)).await?;
                        return Ok(());
                    }
                },
                None => return Ok(()),
            },
            from_city = city.next() => match from_city {
                Some(Ok(CityMessage::Text(text))) => send(device, &conduit.seal_for_device(text.as_str())?).await?,
                Some(Ok(CityMessage::Binary(_) | CityMessage::Ping(_) | CityMessage::Pong(_) | CityMessage::Frame(_))) => {}
                Some(Ok(CityMessage::Close(_)) | Err(_)) | None => return Ok(()),
            },
        }
    }
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
