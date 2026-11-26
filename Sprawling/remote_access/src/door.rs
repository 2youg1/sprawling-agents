// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, TimeMs};

use crate::pairing::PairingCode;

mod spelling;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch([u8; 16]);

impl Epoch {
    #[must_use]
    pub fn from_entropy(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId([u8; 16]);

impl DeviceId {
    #[must_use]
    pub fn from_entropy(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionId([u8; 16]);

impl SessionId {
    #[must_use]
    pub fn from_entropy(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceName(String);

impl DeviceName {
    pub const MAX_CHARS: usize = 64;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    Watch,

    Act,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbClass {
    Read,
    Act,

    LocalOnly,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Door {
    epoch: Epoch,
    phase: Phase,
    pending: Vec<Pending>,
    devices: Vec<Device>,
    sessions: Vec<Session>,
}

impl Door {
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

    pub fn close(&mut self) {
        self.phase = Phase::Closed;
        self.pending.clear();
        self.sessions.clear();
    }

    pub fn tick(&mut self, now: TimeMs) {
        if let Phase::Open { closes_at } = self.phase
            && now >= closes_at
        {
            self.close();
        }
        self.pending.retain(|pending| now < pending.expires);
        self.sessions.retain(|session| now < session.expires);
    }

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

    pub fn end_session(&mut self, session: SessionId) {
        self.sessions.retain(|held| held.id != session);
    }

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

    #[must_use]
    pub fn device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    #[must_use]
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

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
