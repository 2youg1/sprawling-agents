// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use aws_lc_rs::signature::{ED25519, UnparsedPublicKey};
use kernel::{AxCode, AxError, B3Hash, TimeMs};
use tokio::sync::watch;

use crate::answer::{DeviceId, DevicesAnswer};
use crate::auth::Pairing;
use crate::reception::{
    BindFace, BrowserDoor, DeviceKey, Guess, Keys, LABEL_MAX, PairedBrowser, Sessions, decode_hex,
};

use super::pairing::{PairProof, SessionBody};
use super::seat::Seat;

const SIGNED_LABEL: &str = "sprawling local session v1";

pub type KeepBrowsers = Arc<dyn Fn(&[PairedBrowser]) -> Result<(), AxError> + Send + Sync>;

pub type DoorClock = Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>;

pub type DoorEntropy = Arc<dyn Fn(&mut [u8]) -> Result<(), AxError> + Send + Sync>;

#[derive(Clone)]
pub struct DoorSenses {
    pub clock: DoorClock,
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
    keep: KeepBrowsers,
    code: watch::Sender<String>,

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
        let door = BrowserDoor::new(browsers, drawn(&senses)?);
        let (code, _) = watch::channel(door.pairing_code().to_owned());
        let (ended, _) = watch::channel(());
        Ok(Self(Arc::new(Inner {
            held: Mutex::new(Held {
                door,
                waiting: BTreeMap::new(),
            }),
            senses,
            keep,
            code,
            ended,
        })))
    }

    #[must_use]
    pub fn pairing_code(&self) -> watch::Receiver<String> {
        self.0.code.subscribe()
    }

    pub fn issue_open_code(&self) -> Result<OpenCode, AxError> {
        let entropy: [u8; 16] = drawn(&self.0.senses)?;
        let now = (self.0.senses.clock)()?;
        let (tell, redeemed) = mpsc::channel();
        let mut held = self.held();
        let code = held.door.issue_open_code(entropy, now);
        held.waiting.insert(B3Hash::digest(code.as_bytes()), tell);
        Ok(OpenCode { code, redeemed })
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
        proof: &PairProof,
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
        let entropy: [u8; 32] = drawn(&self.0.senses)?;
        let mut held = self.held();
        match proof {
            PairProof::Code(word) => match held.door.guess(word, now, entropy) {
                Guess::Paired => {
                    self.0
                        .code
                        .send_replace(held.door.pairing_code().to_owned());
                }
                Guess::Wrong => {
                    self.0
                        .code
                        .send_replace(held.door.pairing_code().to_owned());
                    return Err(refused("type the pairing code the terminal shows now"));
                }
                Guess::TooSoon => {
                    return Err(refused(
                        "wait a second, then type the code the terminal shows",
                    ));
                }
            },
            PairProof::Open(code) => {
                let Some(digest) = held.door.redeem_open_code(code, now) else {
                    return Err(refused(
                        "open the city again from its terminal, or type the pairing code it shows",
                    ));
                };
                if let Some(tell) = held.waiting.remove(&digest) {
                    let _unheard: Result<(), mpsc::SendError<()>> = tell.send(());
                }
            }
        }
        let line = PairedBrowser::new(key, label, now);
        let mut table: Vec<PairedBrowser> = held
            .door
            .browsers()
            .into_iter()
            .filter(|browser| browser.id != line.id)
            .collect();
        table.push(line.clone());
        (self.0.keep)(&table)?;
        Ok(held.door.pair(line))
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
        let now = (self.0.senses.clock)()?;
        Ok(self.held().door.challenge(entropy, now))
    }

    pub(crate) fn open_session(&self, body: &SessionBody, origin: &str) -> Result<String, AxError> {
        let now = (self.0.senses.clock)()?;
        let entropy: [u8; 32] = drawn(&self.0.senses)?;
        let mut held = self.held();
        if !held.door.take_nonce(&body.nonce, now) {
            return Err(unsigned(
                "ask /session/challenge for a new nonce and sign it",
            ));
        }
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
        let token = held
            .door
            .open_session(&body.device, entropy, now)
            .ok_or_else(|| unsigned("pair this browser again: the city does not know it"))?;

        self.0.ended.send_replace(());
        Ok(token)
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

fn refused(recovery: &str) -> AxError {
    AxError::failure(
        AxCode::PairingRefused,
        "pair a browser",
        "the code was wrong, used, expired or sent too soon",
    )
    .with_recovery(recovery.to_owned())
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
