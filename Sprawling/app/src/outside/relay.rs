// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use kernel::event::record::RemoteClosing;
use kernel::{AxCode, AxError};
use remote_access::handshake::{Finish, Hello, PairHello};
use tokio_tungstenite::tungstenite::Message as CityMessage;

use super::conduit::{Conduit, Step};
use super::keeper::Doorway;
use super::listener::Reaching;
use super::stream::Device;

pub(super) const HANDSHAKE_PATIENCE: Duration = Duration::from_secs(10);

#[derive(Clone, Copy)]
pub(super) enum Run {
    Pair,
    Session,
}

pub(super) async fn carry(mut device: Device, doorway: &Doorway, reaching: &Reaching, run: Run) {
    let ended = match run {
        Run::Pair => pairing(&mut device, doorway).await,
        Run::Session => session(&mut device, doorway, reaching).await,
    };
    let code = ended
        .as_ref()
        .err()
        .map_or("", |refused| refused.code().as_str());
    if let Err(_disconnected) = device.close(code).await {}
}

async fn handshake(device: &mut Device) -> Result<Bytes, AxError> {
    match tokio::time::timeout(HANDSHAKE_PATIENCE, device.recv()).await {
        Ok(Ok(Some(bytes))) => Ok(bytes),
        Ok(Err(source)) => Err(lost(&source)),
        Ok(Ok(None)) | Err(_) => Err(lost(
            &"the device left or did not finish its remote handshake",
        )),
    }
}

async fn send(device: &mut Device, bytes: &[u8]) -> Result<(), AxError> {
    device.send(bytes).await.map_err(|source| lost(&source))
}

async fn pairing(device: &mut Device, doorway: &Doorway) -> Result<(), AxError> {
    let hello = PairHello::from_bytes(&handshake(device).await?)?;
    let (reply, waiting) = doorway.pair_reply(&hello)?;
    send(device, reply.as_bytes()).await?;
    let sealed = handshake(device).await?;
    let door = doorway.clone();
    let answer = blocking(move || door.claim(waiting, &sealed)).await?;
    send(device, &answer).await
}

async fn session(
    device: &mut Device,
    doorway: &Doorway,
    reaching: &Reaching,
) -> Result<(), AxError> {
    let hello = Hello::from_bytes(&handshake(device).await?)?;
    let (reply, waiting) = doorway.session_reply(&hello)?;
    send(device, reply.as_bytes()).await?;
    let finish = Finish::from_bytes(&handshake(device).await?)?;
    let door = doorway.clone();
    let token = reaching.token.clone();
    let mut conduit = blocking(move || {
        let admitted = door.admit(waiting, &finish)?;
        Ok(Conduit::new(door, admitted, token))
    })
    .await?;
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
    loop {
        tokio::select! {
            from_device = device.recv() => match from_device.map_err(|source| lost(&source))? {
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
