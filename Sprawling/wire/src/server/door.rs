// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use aws_lc_rs::hmac;
use aws_lc_rs::signature::{ED25519, UnparsedPublicKey};
use kernel::{AxCode, AxError, B3Hash, TimeMs};
use tokio::sync::watch;

use crate::answer::{DeviceId, DevicesAnswer};
use crate::auth::Pairing;
use crate::reception::{
    BindFace, BrowserDoor, Challenge, DeviceKey, Keys, LABEL_MAX, PairedBrowser, Sessions,
    decode_hex,
};

use super::pairing::{RecoverySessionBody, SessionBody};
use super::seat::Seat;

const SIGNED_LABEL: &str = "sprawling local session v1";

pub type KeepBrowsers = Arc<dyn Fn(&[PairedBrowser]) -> Result<(), AxError> + Send + Sync>;

pub type DoorClock = Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>;

pub type DoorEntropy = Arc<dyn Fn(&mut [u8]) -> Result<(), AxError> + Send + Sync>;

#[derive(Clone)]
pub struct DoorSenses {
    pub clock: DoorClock,
    pub monotonic: Arc<dyn Fn() -> Instant + Send + Sync>,
    pub entropy: DoorEntropy,
}

pub struct OpenCode {
    pub code: String,

    pub redeemed: mpsc::Receiver<()>,
}

#[derive(Clone)]
pub struct LocalDoor(Arc<Inner>);

struct Inner {
    held: Mutex<Held>,
    senses: DoorSenses,
    challenge_key: hmac::Key,
    started: Instant,
    keep: KeepBrowsers,
    ended: watch::Sender<()>,
}

struct Held {
    door: BrowserDoor,
    waiting: BTreeMap<B3Hash, mpsc::Sender<()>>,
}

impl LocalDoor {
    pub fn new(
        browsers: Vec<PairedBrowser>,
        senses: DoorSenses,
        keep: KeepBrowsers,
    ) -> Result<Self, AxError> {
        let door = BrowserDoor::new(browsers);
        let secret: [u8; 32] = drawn(&senses)?;
        let challenge_key = hmac::Key::new(hmac::HMAC_SHA256, &secret);
        let started = (senses.monotonic)();
        let (ended, _) = watch::channel(());
        Ok(Self(Arc::new(Inner {
            held: Mutex::new(Held {
                door,
                waiting: BTreeMap::new(),
            }),
            senses,
            challenge_key,
            started,
            keep,
            ended,
        })))
    }

    pub fn issue_open_code(&self) -> Result<OpenCode, AxError> {
        let (tell, redeemed) = mpsc::channel();
        let code = self.issue_code(Some(tell))?;
        Ok(OpenCode { code, redeemed })
    }

    pub fn issue_display_code(&self) -> Result<String, AxError> {
        self.issue_code(None)
    }

    fn issue_code(&self, tell: Option<mpsc::Sender<()>>) -> Result<String, AxError> {
        let entropy = drawn(&self.0.senses)?;
        let now = self.elapsed_ms();
        let mut held = self.held();
        let Held { door, waiting } = &mut *held;
        let (code, digest) = door.issue_open_code(entropy, now);
        waiting.retain(|held, _| door.has_open_code(held, now));
        if let Some(tell) = tell {
            waiting.insert(digest, tell);
        }
        Ok(code)
    }

    #[must_use]
    pub fn devices(&self) -> DevicesAnswer {
        self.held().door.devices()
    }

    pub fn forget(&self, device: &DeviceId) -> Result<bool, AxError> {
        let mut held = self.held();
        let table: Vec<PairedBrowser> = held
            .door
            .browsers()
            .into_iter()
            .filter(|browser| &browser.id != device)
            .collect();
        (self.0.keep)(&table)?;
        let forgotten = held.door.forget(device);
        self.0.ended.send_replace(());
        Ok(forgotten)
    }

    pub(crate) fn with_sessions<R>(
        &self,
        judge: impl FnOnce(&Sessions, TimeMs) -> R,
    ) -> Result<R, AxError> {
        let now = (self.0.senses.clock)()?;
        Ok(judge(self.held().door.sessions(), now))
    }

    pub(crate) fn pairing(
        &self,
        face: &BindFace,
        offered: Option<&str>,
    ) -> Result<Pairing, AxError> {
        self.with_sessions(|sessions, now| {
            Keys {
                face,
                sessions,
                now,
            }
            .pairing(offered)
        })
    }

    pub(crate) fn seat(&self, presented: Option<String>) -> Result<Seat, AxError> {
        let now = (self.0.senses.clock)()?;
        let mut held = self.held();
        let mut ended = self.0.ended.subscribe();
        ended.mark_changed();
        let token = presented
            .as_deref()
            .and_then(|token| held.door.sessions_mut().seat(token, now));
        Ok(Seat {
            door: self.clone(),
            presented,
            token,
            ended,
        })
    }

    pub(crate) fn pair(
        &self,
        code: &str,
        public_key: &str,
        label: &str,
    ) -> Result<DeviceId, AxError> {
        let key = DeviceKey::parse(public_key).ok_or_else(|| {
            invalid(
                "public_key",
                "send the 32 bytes of the Ed25519 public key as 64 hex characters",
            )
        })?;
        let label = readable_label(label)?;
        let now = (self.0.senses.clock)()?;
        let mut held = self.held();
        let digest = held
            .door
            .open_code(code, self.elapsed_ms())
            .ok_or_else(refused)?;
        let line = PairedBrowser::new(key, label, now);
        let mut table: Vec<PairedBrowser> = held
            .door
            .browsers()
            .into_iter()
            .filter(|browser| browser.id != line.id)
            .collect();
        table.push(line.clone());
        (self.0.keep)(&table)?;
        let device = held.door.pair(line);
        held.door.spend_open_code(&digest);
        if let Some(tell) = held.waiting.remove(&digest) {
            let _unheard = tell.send(());
        }
        Ok(device)
    }

    pub(super) fn unseat(&self, token: &B3Hash) {
        let now = (self.0.senses.clock)();
        let mut held = self.held();
        match now {
            Ok(now) => held.door.sessions_mut().unseat(token, now),

            Err(_) => held.door.sessions_mut().end(token),
        }
    }

    pub(crate) fn challenge(&self) -> Result<String, AxError> {
        let entropy: [u8; 32] = drawn(&self.0.senses)?;
        Ok(Challenge::issue(
            &self.0.challenge_key,
            entropy,
            self.elapsed_ms(),
        ))
    }

    pub(crate) fn open_session(&self, body: &SessionBody, origin: &str) -> Result<String, AxError> {
        let renew = || unsigned("ask /session/challenge for a new nonce and sign it");
        let challenge = Challenge::read(&self.0.challenge_key, &body.nonce, self.elapsed_ms())
            .ok_or_else(renew)?;
        let now = (self.0.senses.clock)()?;
        let mut held = self.held();
        let Some(key) = held.door.device_key(&body.device).copied() else {
            return Err(unsigned(
                "pair this browser again: the city does not know it",
            ));
        };
        let signature: [u8; 64] = decode_hex(&body.signature)
            .ok_or_else(|| unsigned("send the 64-byte Ed25519 signature as 128 hex characters"))?;
        let signed = format!("{SIGNED_LABEL}\n{}\n{origin}", body.nonce);
        UnparsedPublicKey::new(&ED25519, key.bytes())
            .verify(signed.as_bytes(), &signature)
            .map_err(|_| unsigned("pair this browser again: its signature does not verify"))?;
        let entropy: [u8; 32] = drawn(&self.0.senses)?;
        if !held
            .door
            .redeem_challenge(challenge, &body.device, self.elapsed_ms())
        {
            return Err(renew());
        }
        let token = held
            .door
            .open_session(&body.device, entropy, now)
            .ok_or_else(|| unsigned("pair this browser again: the city does not know it"))?;

        self.0.ended.send_replace(());
        Ok(token)
    }

    pub(super) fn recover_session(
        &self,
        body: RecoverySessionBody,
        origin: &str,
    ) -> Result<(DeviceId, String), AxError> {
        let key = DeviceKey::parse(&body.public_key).ok_or_else(|| {
            invalid(
                "public_key",
                "send the 32 bytes of the Ed25519 public key as 64 hex characters",
            )
        })?;
        let request = SessionBody {
            device: key.id(),
            nonce: body.nonce,
            signature: body.signature,
        };
        let token = self.open_session(&request, origin)?;
        Ok((request.device, token))
    }

    fn elapsed_ms(&self) -> u64 {
        let elapsed = (self.0.senses.monotonic)().saturating_duration_since(self.0.started);
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
    }

    fn held(&self) -> MutexGuard<'_, Held> {
        self.0.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn drawn<const N: usize>(senses: &DoorSenses) -> Result<[u8; N], AxError> {
    let mut bytes = [0u8; N];
    (senses.entropy)(&mut bytes)?;
    Ok(bytes)
}

fn readable_label(label: &str) -> Result<String, AxError> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > LABEL_MAX || label.chars().any(char::is_control)
    {
        return Err(invalid(
            "label",
            "name the browser in 1 to 64 characters, with no control characters",
        ));
    }
    Ok(label.to_owned())
}

fn refused() -> AxError {
    AxError::failure(
        AxCode::PairingRefused,
        "pair a browser",
        "the code was wrong, used or expired",
    )
    .with_recovery("hold Ctrl+G in the quiet host, click the code to copy it, and paste the fresh pairing code")
}

fn unsigned(recovery: &str) -> AxError {
    AxError::failure(
        AxCode::GateDenied,
        "open a browser session",
        "the challenge was not answered by a paired browser",
    )
    .with_recovery(recovery.to_owned())
}

fn invalid(field: &str, recovery: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "pair a browser",
        format!("`{field}` cannot be read"),
    )
    .with_recovery(recovery.to_owned())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::disallowed_methods,
    reason = "test clock and fixtures"
)]
mod tests {
    use super::*;
    use crate::reception::encode_hex;
    use aws_lc_rs::signature::{Ed25519KeyPair, KeyPair};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    const ORIGIN: &str = "http://localhost:7341";

    fn paired_door() -> (LocalDoor, Ed25519KeyPair, DeviceId, Arc<AtomicU64>) {
        let signer = Ed25519KeyPair::from_seed_unchecked(&[8; 32]).expect("device seed");
        let key = DeviceKey::parse(&encode_hex(signer.public_key().as_ref())).expect("public key");
        let device = key.id();
        let now = Arc::new(AtomicU64::new(100));
        let clock = Arc::clone(&now);
        let elapsed = Arc::clone(&now);
        let start = Instant::now();
        let next = AtomicU64::new(0);
        let senses = DoorSenses {
            clock: Arc::new(move || Ok(TimeMs::new(clock.load(Ordering::Relaxed)))),
            monotonic: Arc::new(move || {
                start
                    .checked_add(Duration::from_millis(elapsed.load(Ordering::Relaxed)))
                    .expect("fixture elapsed time fits the test clock")
            }),
            entropy: Arc::new(move |bytes| {
                for (byte, lane) in bytes.iter_mut().zip(
                    next.fetch_add(1, Ordering::Relaxed)
                        .to_be_bytes()
                        .into_iter()
                        .cycle(),
                ) {
                    *byte = lane;
                }
                Ok(())
            }),
        };
        let door = LocalDoor::new(
            vec![PairedBrowser::new(
                key,
                "browser".to_owned(),
                TimeMs::new(0),
            )],
            senses,
            Arc::new(|_| Ok(())),
        )
        .expect("local door");
        (door, signer, device, now)
    }

    fn answer(signer: &Ed25519KeyPair, device: DeviceId, nonce: String) -> SessionBody {
        let signed = format!("{SIGNED_LABEL}\n{nonce}\n{ORIGIN}");
        let signature = encode_hex(signer.sign(signed.as_bytes()).as_ref());
        SessionBody {
            device,
            nonce,
            signature,
        }
    }

    #[test]
    fn challenge_requests_cannot_displace_a_pending_browser() {
        let (door, signer, device, _) = paired_door();
        let nonce = door.challenge().expect("pending browser challenge");
        for _ in 0..1_000 {
            door.challenge().expect("another browser challenge");
        }
        let body = answer(&signer, device, nonce);
        assert!(door.open_session(&body, ORIGIN).is_ok());
        assert!(door.open_session(&body, ORIGIN).is_err());
    }

    #[test]
    fn invalid_signatures_do_not_consume_the_challenge() {
        let (door, signer, device, _) = paired_door();
        let nonce = door.challenge().expect("pending browser challenge");
        let valid = answer(&signer, device.clone(), nonce.clone());
        let invalid = SessionBody {
            device,
            nonce,
            signature: encode_hex(&[0; 64]),
        };
        assert!(door.open_session(&invalid, ORIGIN).is_err());
        assert!(door.open_session(&valid, ORIGIN).is_ok());
    }

    #[test]
    fn modified_and_expired_challenges_are_refused() {
        let (door, signer, device, clock) = paired_door();
        let nonce = door.challenge().expect("pending browser challenge");
        let modified = nonce.replacen('0', "1", 1);
        assert_ne!(nonce, modified);
        let forged = answer(&signer, device.clone(), modified);
        assert!(door.open_session(&forged, ORIGIN).is_err());
        clock.store(60_100, Ordering::Relaxed);
        let expired = answer(&signer, device, nonce);
        assert!(door.open_session(&expired, ORIGIN).is_err());
    }
}
