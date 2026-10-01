// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door as this process keeps it (sprawling-SPEC.md 8-139):
//! the one `remote_access::door::Door`, the city's signing key, the
//! route, the device table and the ledger lines, behind one lock.
//!
//! [`Doorway`] is the handle every caller shares - the console's thread
//! and each connection the remote listener answers - so a state change
//! and the line that records it happen under the same lock and in the
//! same order. The door decides; this module carries out what it
//! decided: it writes the five lines (`crates/kernel/Spec.lean` §8-81),
//! keeps the device table on disk, and opens and closes the route.
//!
//! Time and randomness come in as [`Senses`], made in `bin::assembly`,
//! so a test drives the door on a counted clock.
//!
//! The point a reader most often gets wrong: the city's signing key is
//! drawn when the process starts and lives only in it. A device pins the
//! key it paired with, so a restart is a new key and every device pairs
//! again; keeping the key across restarts is the vault's work, which
//! sprawling-SPEC.md 8-139 leaves open.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use kernel::event::Who;
use kernel::event::record::{DeviceRevoked, RemoteClosed, RemoteClosing, RemoteOpened};
use kernel::{AxCode, AxError, EventDraft, EventKind, Ledger, Payload, RunId, TimeMs};
use remote_access::door::{Device, DeviceName, Door, Epoch};
use remote_access::handshake::NONCE_BYTES;
use remote_access::keys::{SEED_BYTES, SigningKey};
use remote_access::route::{Opened, Route};

use super::console::Lasting;

mod pairing;
mod sessions;

pub(crate) use pairing::Inviting;

/// The wall clock, read where `bin::assembly` says.
pub(crate) type Clock = Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>;
/// This machine's random source, filling the slice it is given.
pub(crate) type Entropy = Arc<dyn Fn(&mut [u8]) -> Result<(), AxError> + Send + Sync>;
/// Builds the route one opening of the door goes out through, closed
/// (sprawling-SPEC.md 8-151): asked once per `open`, so the route is the
/// one the city's configuration names at that moment.
pub(crate) type Choosing = Box<dyn FnMut() -> Result<Box<dyn Route + Send>, AxError> + Send>;

/// What the door reads this machine through.
#[derive(Clone)]
pub(crate) struct Senses {
    pub(crate) clock: Clock,
    pub(crate) entropy: Entropy,
}

/// What a door is kept with, settled before it is.
pub(crate) struct Keeping {
    /// Where the device table lives: `CityLayout::devices`.
    pub(crate) devices: PathBuf,
    /// Where the five lines go: the worker's relay in a served city.
    pub(crate) ledger: Box<dyn Ledger + Send>,
    /// How each opening of the door builds its route.
    pub(crate) choose: Choosing,
    pub(crate) senses: Senses,
}

/// Which paired devices to revoke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Revoking {
    /// Every device with this name.
    Named(String),
    All,
}

/// Whether the door is open after a look at the clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Open,
    Closed,
}

/// The handle the console and every remote connection share.
#[derive(Clone)]
pub(crate) struct Doorway {
    kept: Arc<Mutex<Kept>>,
    senses: Senses,
}

struct Kept {
    door: Door,
    city: SigningKey,
    devices: PathBuf,
    ledger: Box<dyn Ledger + Send>,
    choose: Choosing,
    standing: Option<Standing>,
}

/// What an open door stands on: the open route and its answer, and the
/// listener answering on its loopback port, which ends when this is
/// dropped.
struct Standing {
    route: Box<dyn Route + Send>,
    opened: Opened,
    listening: Option<Box<dyn Send>>,
}

impl Doorway {
    /// The door this process starts with: closed, holding the devices
    /// the table on disk names.
    ///
    /// # Errors
    /// An unreadable device table; a random source that refuses.
    pub(crate) fn keep(keeping: Keeping) -> Result<Doorway, AxError> {
        let Keeping {
            devices,
            ledger,
            choose,
            senses,
        } = keeping;
        let known = super::devices::read(&devices)?;
        let mut seed = [0u8; SEED_BYTES];
        (senses.entropy)(&mut seed)?;
        let city = SigningKey::from_seed(&seed)?;
        let door = Door::start(known, Epoch::from_entropy(drawn(&senses)?));
        Ok(Doorway {
            kept: Arc::new(Mutex::new(Kept {
                door,
                city,
                devices,
                ledger,
                choose,
                standing: None,
            })),
            senses,
        })
    }

    /// Opens the route to `local` and the door for `lasting`, and writes
    /// `remote_opened`.
    ///
    /// # Errors
    /// The door is open already; no route can be built from what the
    /// city chose; the route does not open; the ledger refuses the line,
    /// in which case the route is closed again.
    pub(crate) fn open(&self, local: SocketAddr, lasting: Lasting) -> Result<Opened, AxError> {
        let now = (self.senses.clock)()?;
        let epoch = Epoch::from_entropy(drawn(&self.senses)?);
        let mut kept = self.kept()?;
        if let Some(standing) = &kept.standing {
            return Err(AxError::failure(
                AxCode::Busy,
                "open the remote door",
                format!("it is open at {}", standing.opened.url.as_str()),
            )
            .with_recovery("close it first with `/remote close`, or leave it open"));
        }
        let mut route = (kept.choose)()?;
        let opened = route.open(local)?;
        let closes_at = TimeMs::new(now.value().saturating_add(lasting.ms()));
        let line = RemoteOpened {
            closes_at,
            url: opened.url.as_str().to_owned(),
        };
        if let Err(refused) = kept.record(now, Who::Person, EventKind::RemoteOpened, &line) {
            route.close()?;
            return Err(refused);
        }
        kept.door.open(epoch, closes_at)?;
        kept.standing = Some(Standing {
            route,
            opened: opened.clone(),
            listening: None,
        });
        Ok(opened)
    }

    /// Keeps `listening` for as long as the door stays open, and drops it
    /// at once when the door already closed.
    pub(crate) fn attend(&self, listening: Box<dyn Send>) -> Result<(), AxError> {
        let mut kept = self.kept()?;
        if let Some(standing) = kept.standing.as_mut() {
            standing.listening = Some(listening);
        }
        Ok(())
    }

    /// Closes the door and its route, and writes `remote_closed`. A door
    /// already closed answers `Phase::Closed` and writes nothing.
    ///
    /// # Errors
    /// The ledger refuses the line, or the route does not close.
    pub(crate) fn close(&self, why: RemoteClosing) -> Result<Phase, AxError> {
        let now = (self.senses.clock)()?;
        self.kept()?.close(now, why)?;
        Ok(Phase::Closed)
    }

    /// Closes the door once its time is up, as `Expired`.
    ///
    /// # Errors
    /// As [`Self::close`].
    pub(crate) fn tick(&self) -> Result<Phase, AxError> {
        let now = (self.senses.clock)()?;
        let mut kept = self.kept()?;
        match kept.door.closes_at() {
            Some(closes_at) if now >= closes_at => {
                kept.close(now, RemoteClosing::Expired)?;
                Ok(Phase::Closed)
            }
            Some(_) => {
                kept.door.tick(now);
                Ok(Phase::Open)
            }
            None => Ok(Phase::Closed),
        }
    }

    /// Every paired device.
    ///
    /// # Errors
    /// The lock was poisoned.
    pub(crate) fn devices(&self) -> Result<Vec<Device>, AxError> {
        Ok(self.kept()?.door.devices().to_vec())
    }

    /// Revokes the chosen devices, keeps the table, and writes one
    /// `device_revoked` each. Their sessions end with them.
    ///
    /// # Errors
    /// A name no paired device has; a table that cannot be written.
    pub(crate) fn revoke(&self, which: &Revoking) -> Result<Vec<Device>, AxError> {
        let now = (self.senses.clock)()?;
        let mut kept = self.kept()?;
        let chosen: Vec<Device> = match which {
            Revoking::All => kept.door.devices().to_vec(),
            Revoking::Named(name) => {
                let name = DeviceName::parse(name)?;
                let named: Vec<Device> = kept
                    .door
                    .devices()
                    .iter()
                    .filter(|device| device.name == name)
                    .cloned()
                    .collect();
                if named.is_empty() {
                    return Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "revoke a device",
                        format!("no paired device is called `{}`", name.as_str()),
                    )
                    .with_recovery("list the paired devices with `/remote devices`"));
                }
                named
            }
        };
        for device in &chosen {
            kept.door.revoke(device.id)?;
        }
        super::devices::write(&kept.devices, kept.door.devices())?;
        for device in &chosen {
            let line = DeviceRevoked {
                device: device.id.text(),
                name: device.name.as_str().to_owned(),
            };
            kept.record(now, Who::Person, EventKind::DeviceRevoked, &line)?;
        }
        Ok(chosen)
    }

    fn kept(&self) -> Result<MutexGuard<'_, Kept>, AxError> {
        self.kept.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the remote door",
                "a thread ended in a panic while holding it",
            )
            .with_recovery(
                "restart the city: the door starts closed and the device table is read again",
            )
        })
    }
}

impl Kept {
    fn close(&mut self, now: TimeMs, why: RemoteClosing) -> Result<(), AxError> {
        let Some(Standing {
            mut route,
            listening,
            ..
        }) = self.standing.take()
        else {
            return Ok(());
        };
        self.door.close();
        // The listener ends before the route goes, so no connection is
        // answered on a door already shut.
        drop(listening);
        let closed_route = route.close();
        let who = match why {
            RemoteClosing::Expired => Who::City,
            RemoteClosing::Console | RemoteClosing::Locked => Who::Person,
        };
        self.record(now, who, EventKind::RemoteClosed, &RemoteClosed { why })?;
        closed_route
    }

    fn require_open(&self, action: &str) -> Result<TimeMs, AxError> {
        self.door.closes_at().ok_or_else(|| closed(action))
    }

    fn record<T: serde::Serialize>(
        &mut self,
        t: TimeMs,
        who: Who,
        kind: EventKind,
        line: &T,
    ) -> Result<(), AxError> {
        let draft = EventDraft {
            run: RunId::CITY,
            t,
            who: who.to_string(),
            addr: None,
            kind,
            data: Payload::of(line)?,
            ig: false,
        };
        self.ledger.append(draft).map(drop)
    }
}

fn drawn(senses: &Senses) -> Result<[u8; 16], AxError> {
    let mut bytes = [0u8; 16];
    (senses.entropy)(&mut bytes)?;
    Ok(bytes)
}

fn nonce(senses: &Senses) -> Result<[u8; NONCE_BYTES], AxError> {
    let mut bytes = [0u8; NONCE_BYTES];
    (senses.entropy)(&mut bytes)?;
    Ok(bytes)
}

fn closed(action: &str) -> AxError {
    AxError::failure(AxCode::GateDenied, action, "the remote door is closed")
        .with_recovery("open it on the city's console with `/remote open`")
}
