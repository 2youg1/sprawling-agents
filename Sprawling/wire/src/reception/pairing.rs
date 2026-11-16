// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{B3Hash, TimeMs};

use super::sessions::Sessions;
use crate::answer::{DeviceId, DeviceLine, DevicesAnswer};
use crate::auth;

pub const GUESS_INTERVAL_MS: u64 = 1_000;

pub const OPEN_CODE_LIFETIME_MS: u64 = 120_000;

pub const OPEN_CODES_MAX: usize = 4;

pub const NONCE_LIFETIME_MS: u64 = 60_000;

pub const NONCES_MAX: usize = 16;

pub const LABEL_MAX: usize = 64;

const CODE_GROUP_LEN: usize = 4;
const CODE_GROUPS: usize = 2;

const DEVICE_ID_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guess {

    Paired,

    Wrong,

    TooSoon,
}

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
    code: String,
    next_guess: TimeMs,
    open: BTreeMap<B3Hash, TimeMs>,
    nonces: BTreeMap<String, TimeMs>,
    browsers: BTreeMap<DeviceId, PairedBrowser>,
    sessions: Sessions,
}

impl BrowserDoor {

    #[must_use]
    pub fn new(browsers: Vec<PairedBrowser>, entropy: [u8; 32]) -> Self {
        Self {
            code: code_of(&entropy),
            next_guess: TimeMs::new(0),
            open: BTreeMap::new(),
            nonces: BTreeMap::new(),
            browsers: browsers
                .into_iter()
                .map(|browser| (browser.id.clone(), browser))
                .collect(),
            sessions: Sessions::default(),
        }
    }

    #[must_use]
    pub fn pairing_code(&self) -> &str {
        &self.code
    }

    pub fn guess(&mut self, word: &str, now: TimeMs, entropy: [u8; 32]) -> Guess {
        if now < self.next_guess {
            return Guess::TooSoon;
        }
        let paired = auth::verify(Some(word.trim()), &B3Hash::digest(self.code.as_bytes()));
        self.code = code_of(&entropy);
        self.next_guess = TimeMs::new(now.value().saturating_add(GUESS_INTERVAL_MS));
        if paired { Guess::Paired } else { Guess::Wrong }
    }

    pub fn issue_open_code(&mut self, entropy: [u8; 16], now: TimeMs) -> String {
        self.open.retain(|_, expires| *expires > now);
        evict_oldest(&mut self.open, OPEN_CODES_MAX);
        let code = encode_hex(&entropy);
        let expires = TimeMs::new(now.value().saturating_add(OPEN_CODE_LIFETIME_MS));
        self.open.insert(B3Hash::digest(code.as_bytes()), expires);
        code
    }

    pub fn redeem_open_code(&mut self, code: &str, now: TimeMs) -> Option<B3Hash> {
        let digest = B3Hash::digest(code.trim().as_bytes());
        let expires = self.open.remove(&digest)?;
        (expires > now).then_some(digest)
    }

    pub fn pair(&mut self, line: PairedBrowser) -> DeviceId {
        let id = line.id.clone();
        self.browsers.insert(id.clone(), line);
        id
    }

    pub fn challenge(&mut self, entropy: [u8; 32], now: TimeMs) -> String {
        self.nonces.retain(|_, expires| *expires > now);
        evict_oldest(&mut self.nonces, NONCES_MAX);
        let nonce = encode_hex(&entropy);
        let expires = TimeMs::new(now.value().saturating_add(NONCE_LIFETIME_MS));
        self.nonces.insert(nonce.clone(), expires);
        nonce
    }

    pub fn take_nonce(&mut self, nonce: &str, now: TimeMs) -> bool {
        self.nonces
            .remove(nonce)
            .is_some_and(|expires| expires > now)
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

fn code_of(entropy: &[u8; 32]) -> String {
    auth::spell(entropy, CODE_GROUP_LEN, CODE_GROUPS)
}

fn evict_oldest<K: Ord + Clone>(entries: &mut BTreeMap<K, TimeMs>, max: usize) {
    while entries.len() >= max {
        let Some(first) = entries
            .iter()
            .min_by_key(|(_, expires)| **expires)
            .map(|(key, _)| key.clone())
        else {
            return;
        };
        entries.remove(&first);
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
    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    struct Attempt {
        after_ms: u64,
        right: bool,
        entropy: [u8; 32],
    }

    fn attempt() -> impl Strategy<Value = Attempt> {
        (0u64..2_500, any::<bool>(), any::<[u8; 32]>()).prop_map(|(after_ms, right, entropy)| {
            Attempt {
                after_ms,
                right,
                entropy,
            }
        })
    }

    proptest! {

        #[test]
        fn every_trace_judges_each_code_once_and_a_second_apart(
            first in any::<[u8; 32]>(),
            attempts in proptest::collection::vec(attempt(), 0..40),
        ) {
            let mut door = BrowserDoor::new(Vec::new(), first);
            let mut now = 0u64;
            let mut last_judged: Option<u64> = None;
            let mut expected_code = code_of(&first);
            for step in attempts {
                now += step.after_ms;
                let shown = door.pairing_code().to_owned();
                prop_assert_eq!(&shown, &expected_code);
                let word = if step.right { shown.clone() } else { format!("{shown}x") };
                let verdict = door.guess(&word, TimeMs::new(now), step.entropy);
                let early = last_judged.is_some_and(|at| now < at + GUESS_INTERVAL_MS);
                if early {
                    prop_assert_eq!(verdict, Guess::TooSoon);
                    prop_assert_eq!(door.pairing_code(), shown.as_str());
                    continue;
                }
                prop_assert_eq!(verdict, if step.right { Guess::Paired } else { Guess::Wrong });
                if let Some(at) = last_judged {
                    prop_assert!(now >= at + GUESS_INTERVAL_MS);
                }
                last_judged = Some(now);
                expected_code = code_of(&step.entropy);
                prop_assert_eq!(door.pairing_code(), expected_code.as_str());
            }
        }
    }

    #[test]
    fn an_open_code_is_redeemed_once_and_not_after_it_expires() {
        let mut door = BrowserDoor::new(Vec::new(), [1; 32]);
        let code = door.issue_open_code([2; 16], TimeMs::new(0));
        assert!(door.redeem_open_code(&code, TimeMs::new(10)).is_some());
        assert!(door.redeem_open_code(&code, TimeMs::new(11)).is_none());
        let late = door.issue_open_code([3; 16], TimeMs::new(0));
        assert!(
            door.redeem_open_code(&late, TimeMs::new(OPEN_CODE_LIFETIME_MS))
                .is_none()
        );
    }

    #[test]
    fn hex_reads_back_what_it_wrote() {
        let bytes = [0x00, 0x7f, 0xa5, 0xff];
        assert_eq!(decode_hex::<4>(&encode_hex(&bytes)), Some(bytes));
        assert_eq!(decode_hex::<4>("00ff"), None);
        assert_eq!(decode_hex::<2>("zz00"), None);
    }
}
