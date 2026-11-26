// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use aws_lc_rs::hmac;
use kernel::{B3Hash, TimeMs};

use super::sessions::Sessions;
use crate::answer::{DeviceId, DeviceLine, DevicesAnswer};
pub const OPEN_CODE_LIFETIME_MS: u64 = 120_000;

pub const OPEN_CODES_MAX: usize = 4;

pub const NONCE_LIFETIME_MS: u64 = 60_000;

const CHALLENGES_PER_BROWSER: usize = 64;

const CHALLENGE_LABEL: &[u8; 28] = b"sprawling local challenge v1";

const CHALLENGE_BYTES: usize = 8 + 32 + 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Challenge {
    expires: u64,
    nonce: [u8; 32],
}

impl Challenge {
    pub(crate) fn issue(key: &hmac::Key, nonce: [u8; 32], now: u64) -> String {
        let challenge = Self {
            expires: now.saturating_add(NONCE_LIFETIME_MS),
            nonce,
        };
        let tag = hmac::sign(key, &challenge.signed());
        encode_hex(
            &[
                challenge.expires.to_be_bytes().as_slice(),
                &challenge.nonce,
                tag.as_ref(),
            ]
            .concat(),
        )
    }

    pub(crate) fn read(key: &hmac::Key, encoded: &str, now: u64) -> Option<Self> {
        if encoded.len() != CHALLENGE_BYTES * 2
            || encoded.bytes().any(|byte| byte.is_ascii_uppercase())
        {
            return None;
        }
        let bytes = decode_hex::<CHALLENGE_BYTES>(encoded)?;
        let (expires, rest) = bytes.split_first_chunk::<8>()?;
        let (nonce, tag) = rest.split_first_chunk::<32>()?;
        let challenge = Self {
            expires: u64::from_be_bytes(*expires),
            nonce: *nonce,
        };
        if now >= challenge.expires {
            return None;
        }
        match hmac::verify(key, &challenge.signed(), tag) {
            Ok(()) => Some(challenge),
            Err(_) => None,
        }
    }

    fn signed(self) -> [u8; 28 + 8 + 32] {
        let mut signed = [0; 28 + 8 + 32];
        let (label, rest) = signed.split_at_mut(CHALLENGE_LABEL.len());
        let (expires, nonce) = rest.split_at_mut(size_of::<u64>());
        label.copy_from_slice(CHALLENGE_LABEL);
        expires.copy_from_slice(&self.expires.to_be_bytes());
        nonce.copy_from_slice(&self.nonce);
        signed
    }
}

pub const LABEL_MAX: usize = 64;

const DEVICE_ID_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceKey([u8; 32]);

impl DeviceKey {
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        decode_hex(text).map(Self)
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn hex(&self) -> String {
        encode_hex(&self.0)
    }

    #[must_use]
    pub fn id(&self) -> DeviceId {
        let digest = encode_hex(B3Hash::digest(&self.0).as_bytes());
        DeviceId::new(digest.chars().take(DEVICE_ID_LEN).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedBrowser {
    pub id: DeviceId,
    pub label: String,
    pub key: DeviceKey,
    pub paired_at: TimeMs,
    pub last_seen: Option<TimeMs>,
}

impl PairedBrowser {
    #[must_use]
    pub fn new(key: DeviceKey, label: String, now: TimeMs) -> Self {
        Self {
            id: key.id(),
            label,
            key,
            paired_at: now,
            last_seen: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BrowserDoor {
    open: BTreeMap<B3Hash, u64>,
    spent: BTreeMap<Challenge, DeviceId>,
    browsers: BTreeMap<DeviceId, PairedBrowser>,
    sessions: Sessions,
}

impl BrowserDoor {
    #[must_use]
    pub fn new(browsers: Vec<PairedBrowser>) -> Self {
        Self {
            open: BTreeMap::new(),
            spent: BTreeMap::new(),
            browsers: browsers
                .into_iter()
                .map(|browser| (browser.id.clone(), browser))
                .collect(),
            sessions: Sessions::default(),
        }
    }

    pub fn issue_open_code(&mut self, entropy: [u8; 16], now: u64) -> (String, B3Hash) {
        self.open.retain(|_, expires| *expires > now);
        if self.open.len() >= OPEN_CODES_MAX
            && let Some(oldest) = self
                .open
                .iter()
                .min_by_key(|(_, expires)| **expires)
                .map(|(digest, _)| *digest)
        {
            self.spend_open_code(&oldest);
        }
        let code = encode_hex(&entropy);
        let digest = B3Hash::digest(&entropy);
        let expires = now.saturating_add(OPEN_CODE_LIFETIME_MS);
        self.open.insert(digest, expires);
        (code, digest)
    }

    pub fn open_code(&self, code: &str, now: u64) -> Option<B3Hash> {
        let digest = B3Hash::digest(&decode_hex::<16>(code)?);
        self.has_open_code(&digest, now).then_some(digest)
    }

    pub fn has_open_code(&self, digest: &B3Hash, now: u64) -> bool {
        self.open.get(digest).is_some_and(|expires| *expires > now)
    }

    pub fn spend_open_code(&mut self, digest: &B3Hash) {
        self.open.remove(digest);
    }

    pub fn pair(&mut self, line: PairedBrowser) -> DeviceId {
        let id = line.id.clone();
        self.browsers.insert(id.clone(), line);
        id
    }

    pub(crate) fn redeem_challenge(
        &mut self,
        challenge: Challenge,
        device: &DeviceId,
        now: u64,
    ) -> bool {
        while self
            .spent
            .first_key_value()
            .is_some_and(|(ticket, _)| ticket.expires <= now)
        {
            drop(self.spent.pop_first());
        }
        if challenge.expires <= now || self.spent.contains_key(&challenge) {
            return false;
        }
        if self.spent.values().filter(|owner| *owner == device).count() >= CHALLENGES_PER_BROWSER {
            return false;
        }
        self.spent.insert(challenge, device.clone());
        true
    }

    #[must_use]
    pub fn device_key(&self, device: &DeviceId) -> Option<&DeviceKey> {
        self.browsers.get(device).map(|browser| &browser.key)
    }

    pub fn open_session(
        &mut self,
        device: &DeviceId,
        entropy: [u8; 32],
        now: TimeMs,
    ) -> Option<String> {
        let browser = self.browsers.get_mut(device)?;
        browser.last_seen = Some(now);
        let token = encode_hex(&entropy);
        self.sessions.mint(device.clone(), &token, now);
        Some(token)
    }

    #[must_use]
    pub fn sessions(&self) -> &Sessions {
        &self.sessions
    }

    pub fn sessions_mut(&mut self) -> &mut Sessions {
        &mut self.sessions
    }

    pub fn forget(&mut self, device: &DeviceId) -> bool {
        self.sessions.end_device(device);
        self.browsers.remove(device).is_some()
    }

    #[must_use]
    pub fn browsers(&self) -> Vec<PairedBrowser> {
        self.browsers.values().cloned().collect()
    }

    #[must_use]
    pub fn devices(&self) -> DevicesAnswer {
        DevicesAnswer {
            devices: self
                .browsers
                .values()
                .map(|browser| DeviceLine {
                    id: browser.id.clone(),
                    label: browser.label.clone(),
                    paired_at: browser.paired_at,
                    last_seen: browser.last_seen,
                })
                .collect(),
        }
    }
}

#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::new();
    for byte in bytes {
        for nibble in [byte.wrapping_shr(4), byte & 0x0f] {
            if let Some(digit) = DIGITS.get(usize::from(nibble)) {
                text.push(char::from(*digit));
            }
        }
    }
    text
}

#[must_use]
pub fn decode_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let digits = text.as_bytes();
    if digits.len() != N.checked_mul(2)? {
        return None;
    }
    let (pairs, rest) = digits.as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let mut bytes = [0u8; N];
    for (slot, [high, low]) in bytes.iter_mut().zip(pairs) {
        let value = |digit: u8| {
            char::from(digit)
                .to_digit(16)
                .and_then(|v| u8::try_from(v).ok())
        };
        *slot = value(*high)?.checked_mul(16)?.checked_add(value(*low)?)?;
    }
    Some(bytes)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn an_open_code_is_redeemed_once_and_not_after_it_expires() {
        let mut door = BrowserDoor::new(Vec::new());
        let (code, digest) = door.issue_open_code([2; 16], 0);
        assert!(door.open_code(&encode_hex(&[3; 16]), 9).is_none());
        assert_eq!(door.open_code(&code, 10), Some(digest));
        door.spend_open_code(&digest);
        assert!(door.open_code(&code, 11).is_none());
        let (late, _) = door.issue_open_code([3; 16], 0);
        assert!(door.open_code(&late, OPEN_CODE_LIFETIME_MS).is_none());
    }

    #[test]
    fn hex_reads_back_what_it_wrote() {
        let bytes = [0x00, 0x7f, 0xa5, 0xff];
        assert_eq!(decode_hex::<4>(&encode_hex(&bytes)), Some(bytes));
        assert_eq!(decode_hex::<4>("00ff"), None);
        assert_eq!(decode_hex::<2>("zz00"), None);
    }
}
