// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The door's state: open or closed and in which epoch, the pairings
//! still pending, the devices paired, and the sessions they hold
//! (remote-SPEC.md §8-1).
//!
//! Specified by `adversary/design/RemoteDoor.lean`, which proves the six
//! properties this module keeps: a closed door admits nothing, closing
//! ends every session, a code is used once in its own epoch before it
//! expires, a revoked device holds nothing, a session never outlives the
//! door, and a remote session never reaches a local-only verb.
//!
//! Time and entropy are parameters. The assembly layer samples the clock
//! and draws the random bytes, so every decision here replays.

use kernel::{AxCode, AxError, TimeMs};

use crate::pairing::PairingCode;

#[cfg(test)]
mod tests;

/// Drawn fresh each time the door opens. Nothing issued in one epoch is
/// honoured in another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch([u8; 16]);

impl Epoch {
    #[must_use]
    pub fn from_entropy(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// A paired device, by the id the city gave it at pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId([u8; 16]);

impl DeviceId {
    #[must_use]
    pub fn from_entropy(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// One session a paired device holds after its handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionId([u8; 16]);

impl SessionId {
    #[must_use]
    pub fn from_entropy(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// The public half of a device's key, as the device sent it at pairing.
/// The handshake reads it; the door only keeps it with the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceKey(Vec<u8>);

impl DeviceKey {
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// What the person called a device when they paired it: `iPad`, `phone`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceName(String);

impl DeviceName {
    /// The longest name kept, in characters.
    pub const MAX_CHARS: usize = 64;

    /// # Errors
    /// Refuses an empty name and one longer than [`Self::MAX_CHARS`].
    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.chars().count() > Self::MAX_CHARS {
            return Err(
                AxError::failure(AxCode::InvalidArgs, "name a device", trimmed).with_recovery(
                    format!(
                        "give the device a name of 1 to {} characters, such as `phone`",
                        Self::MAX_CHARS
                    ),
                ),
            );
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a paired device may do once it holds a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    /// Read the city, and nothing else.
    Watch,
    /// Read, and act on work: dispatch, steer, stop, halt, release, answer.
    Act,
}

/// The three kinds of verb a frame can carry, as the door judges them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbClass {
    Read,
    Act,
    /// Widening access or reaching credentials: opening the door, pairing,
    /// attaching an endpoint, writing rules or configuration.
    LocalOnly,
}

/// Whether a remote session with `authority` may carry a verb of `class`.
/// A local-only verb is refused whatever the authority.
#[must_use]
pub fn permits(authority: Authority, class: VerbClass) -> bool {
    match (authority, class) {
        (Authority::Watch | Authority::Act, VerbClass::Read) | (Authority::Act, VerbClass::Act) => {
            true
        }
        (Authority::Watch, VerbClass::Act)
        | (Authority::Watch | Authority::Act, VerbClass::LocalOnly) => false,
    }
}

/// A device the door knows, kept across restarts by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    pub name: DeviceName,
    pub authority: Authority,
    pub key: DeviceKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Closed,
    Open { closes_at: TimeMs },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    code: PairingCode,
    epoch: Epoch,
    expires: TimeMs,
    authority: Authority,
    name: DeviceName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Session {
    id: SessionId,
    device: DeviceId,
    epoch: Epoch,
    expires: TimeMs,
}

/// The whole state of one city's remote door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Door {
    epoch: Epoch,
    phase: Phase,
    pending: Vec<Pending>,
    devices: Vec<Device>,
    sessions: Vec<Session>,
}

impl Door {
    /// The door a city starts with: closed, holding the devices it paired
    /// before. `epoch` is drawn fresh so that opening later cannot land on
    /// the epoch a previous process used.
    #[must_use]
    pub fn start(devices: Vec<Device>, epoch: Epoch) -> Self {
        Self {
            epoch,
            phase: Phase::Closed,
            pending: Vec::new(),
            devices,
            sessions: Vec::new(),
        }
    }

    /// Opens the door until `closes_at`, in a new epoch. Nothing pending and
    /// no session survives into it.
    ///
    /// # Errors
    /// Refuses to open in the epoch it already stands in, which is the one
    /// way an old code or session could come back.
    pub fn open(&mut self, epoch: Epoch, closes_at: TimeMs) -> Result<(), AxError> {
        if epoch == self.epoch {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "open the remote door",
                "a reused epoch",
            )
            .with_recovery("draw a new epoch from the system's random source and open again"));
        }
        *self = Self {
            epoch,
            phase: Phase::Open { closes_at },
            devices: std::mem::take(&mut self.devices),
            pending: Vec::new(),
            sessions: Vec::new(),
        };
        Ok(())
    }

    /// Closes the door: every pending pairing and every session ends, and
    /// the paired devices stay paired.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
        self.pending.clear();
        self.sessions.clear();
    }

    /// Closes the door once its time is up, and forgets what expired.
    pub fn tick(&mut self, now: TimeMs) {
        if let Phase::Open { closes_at } = self.phase
            && now >= closes_at
        {
            self.close();
        }
        self.pending.retain(|pending| now < pending.expires);
        self.sessions.retain(|session| now < session.expires);
    }

    /// Registers a pairing code for the current epoch; the caller minted it
    /// with [`PairingCode::mint`] and shows its text once.
    ///
    /// # Errors
    /// Refuses while the door is closed.
    pub fn expect_pairing(
        &mut self,
        code: PairingCode,
        name: DeviceName,
        authority: Authority,
        expires: TimeMs,
    ) -> Result<(), AxError> {
        self.require_open("mint a pairing code")?;
        self.pending.push(Pending {
            code,
            epoch: self.epoch,
            expires,
            authority,
            name,
        });
        Ok(())
    }

    /// Pairs the device that presented `code` with its public `key`. The
    /// code is removed whether or not it was still good, so it answers once.
    ///
    /// # Errors
    /// Refuses a closed door, a code this door is not waiting for, one from
    /// an earlier epoch, and one past its expiry.
    pub fn pair(
        &mut self,
        code: PairingCode,
        id: DeviceId,
        key: DeviceKey,
        now: TimeMs,
    ) -> Result<(), AxError> {
        self.require_open("pair a device")?;
        let found = self.pending.iter().position(|pending| pending.code == code);
        let pending = found.map(|at| self.pending.swap_remove(at));
        match pending {
            Some(pending) if pending.epoch == self.epoch && now < pending.expires => {
                self.devices.push(Device {
                    id,
                    name: pending.name,
                    authority: pending.authority,
                    key,
                });
                Ok(())
            }
            Some(_) | None => Err(AxError::failure(
                AxCode::GateDenied,
                "pair a device",
                "the pairing code",
            )
            .with_recovery(
                "mint a new pairing code on the city's console with `/remote pair <name>`",
            )),
        }
    }

    /// Forgets a paired device and ends every session it holds.
    ///
    /// # Errors
    /// Refuses an id no device holds, so a typo is not mistaken for a
    /// revocation.
    pub fn revoke(&mut self, id: DeviceId) -> Result<(), AxError> {
        if !self.devices.iter().any(|device| device.id == id) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "revoke a device",
                "an unknown device",
            )
            .with_recovery("list the paired devices with `/remote devices` and name one of them"));
        }
        self.devices.retain(|device| device.id != id);
        self.sessions.retain(|session| session.device != id);
        Ok(())
    }

    /// Opens a session for a paired device whose handshake proved its key.
    /// The session ends at `expires` or when the door closes, whichever is
    /// first.
    ///
    /// # Errors
    /// Refuses a closed door and a device that is not paired.
    pub fn admit(
        &mut self,
        session: SessionId,
        device: DeviceId,
        expires: TimeMs,
    ) -> Result<(), AxError> {
        let closes_at = self.require_open("open a remote session")?;
        if !self.devices.iter().any(|known| known.id == device) {
            return Err(AxError::failure(
                AxCode::GateDenied,
                "open a remote session",
                "an unpaired device",
            )
            .with_recovery("pair this device again from the city's console"));
        }
        self.sessions.push(Session {
            id: session,
            device,
            epoch: self.epoch,
            expires: expires.min(closes_at),
        });
        Ok(())
    }

    /// The authority a session carries at `now`, or `None` when it is no
    /// longer valid: the door closed, its epoch passed, its device was
    /// revoked, or it expired.
    #[must_use]
    pub fn authority(&self, session: SessionId, now: TimeMs) -> Option<Authority> {
        if matches!(self.phase, Phase::Closed) {
            return None;
        }
        let held = self
            .sessions
            .iter()
            .find(|held| held.id == session && held.epoch == self.epoch && now < held.expires)?;
        self.devices
            .iter()
            .find(|device| device.id == held.device)
            .map(|device| device.authority)
    }

    /// The paired device with `id`, whose key a handshake checks.
    #[must_use]
    pub fn device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// Every paired device, for the caller to keep across restarts.
    #[must_use]
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// When the door closes by itself, or `None` while it is closed.
    #[must_use]
    pub fn closes_at(&self) -> Option<TimeMs> {
        match self.phase {
            Phase::Closed => None,
            Phase::Open { closes_at } => Some(closes_at),
        }
    }

    fn require_open(&self, action: &str) -> Result<TimeMs, AxError> {
        self.closes_at().ok_or_else(|| {
            AxError::failure(AxCode::GateDenied, action, "the remote door is closed")
                .with_recovery("open it on the city's console with `/remote open`")
        })
    }
}
