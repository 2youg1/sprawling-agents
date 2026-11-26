// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::{AxCode, AxError};

use crate::serving::key_file::KeyFile;

pub(super) type Commands =
    Arc<dyn Fn(wire::WireCommand, wire::Reply) -> Result<(), AxError> + Send + Sync>;

pub(super) struct Doorstep {
    pub(super) door: wire::LocalDoor,
    key_file: KeyFile,
}

impl Doorstep {
    pub(super) fn kept(door: wire::LocalDoor, port: u16, key: &str) -> Result<Self, AxError> {
        Ok(Self {
            door,
            key_file: KeyFile::write(port, key)?,
        })
    }

    pub(super) fn close(self) {
        if let Err(left) = self.key_file.remove() {
            eprintln!("{}: {}", left.action(), left.recovery());
        }
    }
}

impl super::Listening {
    #[must_use]
    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.bound.local_addr()
    }

    #[must_use]
    pub fn origins(&self) -> &wire::ListenerOrigins {
        self.bound.origins()
    }

    #[must_use]
    pub fn door(&self) -> &wire::LocalDoor {
        &self.doorstep.door
    }
}

pub(super) fn wired(
    city_root: &std::path::Path,
    commands: Commands,
    views: wire::Answering,
) -> Result<(wire::LocalDoor, Commands, wire::Answering), AxError> {
    let table = kernel::layout::CityLayout::new(city_root).browsers();
    let door = wire::LocalDoor::new(
        crate::serving::browsers::load(&table)?,
        wire::DoorSenses {
            clock: Arc::new(|| accounting::Clock::now(&super::super::SystemClock)),
            monotonic: Arc::new(crate::serving::standing::monotonic_now),
            entropy: Arc::new(|bytes: &mut [u8]| {
                getrandom::fill(bytes).map_err(|refused| {
                    AxError::failure(
                        AxCode::ConfigInvalid,
                        "draw randomness for this machine's door",
                        refused.to_string(),
                    )
                    .with_recovery("this machine's random source refused; restart the city")
                })
            }),
        },
        crate::serving::browsers::keeping(table),
    )?;
    Ok((
        door.clone(),
        forgetting(door.clone(), commands),
        devices_first(door, views),
    ))
}

fn devices_first(door: wire::LocalDoor, views: wire::Answering) -> wire::Answering {
    Arc::new(move |query| {
        if !matches!(query, wire::Query::Devices) {
            return views(query);
        }

        let (at, _) = views(query);
        (at, Ok(wire::Answer::Devices(Box::new(door.devices()))))
    })
}

fn forgetting(door: wire::LocalDoor, desk: Commands) -> Commands {
    Arc::new(move |command, reply| {
        if let wire::Command::ForgetDevice(forgetting) = &command {
            return door.forget(&forgetting.device).map(|_forgotten| ());
        }
        desk(command, reply)
    })
}
