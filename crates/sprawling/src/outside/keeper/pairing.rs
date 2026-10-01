// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A device's first connection, as the door keeper carries it out: the
//! code the console mints, the city's half of the pairing handshake,
//! and the claim that makes a device paired (remote_access-SPEC.md
//! §8-6; sprawling-SPEC.md 8-139).

use kernel::event::Who;
use kernel::event::record::DevicePaired;
use kernel::{AxCode, AxError, EventKind, TimeMs};
use remote_access::door::{Authority, DeviceId, DeviceKey, DeviceName};
use remote_access::handshake::{
    self, CityFingerprint, CityPairing, Invitation, PairHello, PairReply,
};
use remote_access::pairing::{CODE_BYTES, PairingCode};
use remote_access::route::Opened;

use super::{Doorway, closed, drawn, nonce};

/// How long a pairing code waits for its device (remote_access-SPEC.md
/// §14): long enough to find the phone, short enough that a code
/// photographed off a screen is stale by the time it is tried.
const PAIRING_MS: u64 = 10 * 60 * 1000;

/// A pairing code the door now waits for, and how a device reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Inviting {
    /// The address with the invitation in its fragment, which the QR
    /// code carries (remote_access-SPEC.md §8-6).
    pub(crate) link: String,
    /// The code as a person reads it aloud: groups of five.
    pub(crate) shown: String,
    pub(crate) invitation: Invitation,
    pub(crate) opened: Opened,
}

impl Doorway {
    /// Mints a pairing code for a device called `name` and waits for it.
    ///
    /// # Errors
    /// A name that is not one, or that a paired device already has; a
    /// closed door.
    pub(crate) fn invite(&self, name: &str, authority: Authority) -> Result<Inviting, AxError> {
        let name = DeviceName::parse(name)?;
        let now = (self.senses.clock)()?;
        let mut entropy = [0u8; CODE_BYTES];
        (self.senses.entropy)(&mut entropy)?;
        let (code, shown) = PairingCode::mint(entropy);
        let mut kept = self.kept()?;
        let opened = kept
            .standing
            .as_ref()
            .map(|standing| standing.opened.clone())
            .ok_or_else(|| closed("pair a device"))?;
        if kept.door.devices().iter().any(|device| device.name == name) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "pair a device",
                format!("a device is already called `{}`", name.as_str()),
            )
            .with_recovery("choose another name, or `/remote revoke` the device first"));
        }
        let expires = TimeMs::new(now.value().saturating_add(PAIRING_MS));
        kept.door.expect_pairing(code, name, authority, expires)?;
        let invitation = Invitation {
            city: CityFingerprint::of(&kept.city.public()),
            code: shown.chars().filter(|each| *each != '-').collect(),
        };
        let link = format!(
            "{}/#pair={}&city={}",
            opened.url.as_str().trim_end_matches('/'),
            invitation.code,
            invitation.city.text()
        );
        Ok(Inviting {
            link,
            shown,
            invitation,
            opened,
        })
    }

    /// The city's answer to a device's first message on a pairing
    /// connection.
    ///
    /// # Errors
    /// A closed door; a failure inside the handshake.
    pub(crate) fn pair_reply(
        &self,
        hello: &PairHello,
    ) -> Result<(PairReply, CityPairing), AxError> {
        let nonce = nonce(&self.senses)?;
        let kept = self.kept()?;
        kept.require_open("pair a device")?;
        handshake::city_pair_reply(hello, &kept.city, nonce)
    }

    /// Opens a device's sealed claim, pairs it, keeps the table, writes
    /// `device_paired`, and answers the device's new id sealed.
    ///
    /// # Errors
    /// A claim that does not open; a code the door refuses; a table that
    /// cannot be written, in which case the device is not kept.
    pub(crate) fn claim(&self, pairing: CityPairing, sealed: &[u8]) -> Result<Vec<u8>, AxError> {
        let (claim, mut session) = pairing.open_claim(sealed)?;
        let id = DeviceId::from_entropy(drawn(&self.senses)?);
        let now = (self.senses.clock)()?;
        let mut kept = self.kept()?;
        let key = DeviceKey::from_bytes(claim.device_key().as_bytes().to_vec());
        kept.door.pair(claim.code(), id, key, now)?;
        if let Err(refused) = super::super::devices::write(&kept.devices, kept.door.devices()) {
            kept.door.revoke(id)?;
            return Err(refused);
        }
        let line = kept.door.device(id).map(|device| DevicePaired {
            device: id.text(),
            name: device.name.as_str().to_owned(),
            authority: device.authority.word().to_owned(),
        });
        if let Some(line) = line {
            kept.record(now, Who::Person, EventKind::DevicePaired, &line)?;
        }
        session.sealer.seal(id.as_bytes())
    }
}
