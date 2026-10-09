// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a served city keeps at its own door on this machine
//! (`crates/sprawling/spec/Serving.lean`): the browser door, with the
//! clock and random source it is handed here, the paired browsers' table
//! behind it, and the key file native clients read.
//!
//! The door answers its own question (`Query::Devices`) and carries out
//! its own command (`ForgetDevice`), because the paired browsers are not
//! in the Ledger; everything else goes on to the views and the desk.

use std::sync::Arc;

use kernel::{AxCode, AxError};

use crate::serving::key_file::KeyFile;

/// The command sink a socket posts to.
pub(super) type Commands =
    Arc<dyn Fn(wire::WireCommand, wire::Reply) -> Result<(), AxError> + Send + Sync>;

/// The browser door and the key file, kept for as long as the city
/// serves and closed with it.
pub(super) struct Doorstep {
    pub(super) door: wire::LocalDoor,
    key_file: KeyFile,
}

impl Doorstep {
    /// Writes the key file for the listener at `port`.
    ///
    /// # Errors
    /// The key file cannot be written where only this account reads it.
    pub(super) fn kept(door: wire::LocalDoor, port: u16, key: &str) -> Result<Self, AxError> {
        Ok(Self {
            door,
            key_file: KeyFile::write(port, key)?,
        })
    }

    /// Removes the key file, saying so when it cannot.
    pub(super) fn close(self) {
        if let Err(left) = self.key_file.remove() {
            eprintln!("{}: {}", left.action(), left.recovery());
        }
    }
}

impl super::Listening {
    /// The address the city's listener holds: the port the operating
    /// system gave when `serve` was asked for port 0. The banner, the
    /// console and the browser this process opens all read this one.
    #[must_use]
    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.bound.local_addr()
    }

    /// The names the listener answers to; its `url` is the one a person
    /// opens (`crates/wire/spec/Reception/Entry.lean` §8-94).
    #[must_use]
    pub fn origins(&self) -> &wire::ListenerOrigins {
        self.bound.origins()
    }

    /// This machine's door for a browser: the pairing code the terminal
    /// shows, and the open codes `/web` hands over.
    #[must_use]
    pub fn door(&self) -> &wire::LocalDoor {
        &self.doorstep.door
    }
}

/// The browser door of the city at `city_root`, knowing every browser
/// paired before, and the two sinks with the door's own verb and question
/// taken out of them.
///
/// # Errors
/// The table of paired browsers cannot be read, or the random source
/// refuses.
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

/// The paired browsers are the door's, not the Ledger's, so their one
/// question is answered there; every other question goes to the views.
fn devices_first(door: wire::LocalDoor, views: wire::Answering) -> wire::Answering {
    Arc::new(move |query| {
        if !matches!(query, wire::Query::Devices) {
            return views(query);
        }
        // The views say where the Ledger stood, which the page reads every
        // answer against; what they say about devices is not theirs.
        let (at, _) = views(query);
        (at, Ok(wire::Answer::Devices(Box::new(door.devices()))))
    })
}

/// Forgetting a paired browser is the door's act, so it is carried out
/// here; every other command goes on to the desk.
fn forgetting(door: wire::LocalDoor, desk: Commands) -> Commands {
    Arc::new(move |command, reply| {
        if let wire::Command::ForgetDevice(forgetting) = &command {
            return door.forget(&forgetting.device).map(|_forgotten| ());
        }
        desk(command, reply)
    })
}
