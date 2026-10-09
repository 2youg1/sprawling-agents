// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The handle on this machine's door for a browser
//! (`crates/wire/spec/Server.lean` §8-93): one lock around
//! [`BrowserDoor`], the clock and random source the assembly layer hands
//! in, and the closure the device table is kept through.
//!
//! Every rule is the state machines' (`reception::pairing`,
//! `reception::sessions`); what is left here is drawing entropy and time
//! for them, checking a signature, telling whoever handed out an open code
//! that it was redeemed, and telling every socket when a session token may
//! have ended, so a forgotten browser loses the sockets it holds
//! (`crates/wire/spec/Server/Sessions.lean` §8-93s).

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

/// The label every session signature starts with, so a signature made
/// for this door is not one any other protocol accepts.
const SIGNED_LABEL: &str = "sprawling local session v1";

/// Where the door keeps its table: the whole table, after every change.
pub type KeepBrowsers = Arc<dyn Fn(&[PairedBrowser]) -> Result<(), AxError> + Send + Sync>;

/// The clock the door reads the time from.
pub type DoorClock = Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>;

/// The random source the door fills its codes, nonces and tokens from.
pub type DoorEntropy = Arc<dyn Fn(&mut [u8]) -> Result<(), AxError> + Send + Sync>;

/// The clock and the random source the door is handed; it samples neither
/// itself.
#[derive(Clone)]
pub struct DoorSenses {
    pub clock: DoorClock,
    pub entropy: DoorEntropy,
}

/// An open code on its way to a browser, and the signal that it arrived.
pub struct OpenCode {
    pub code: String,
    /// Receives once when the code is redeemed. Dropped unanswered when
    /// the code expired or the door went away.
    pub redeemed: mpsc::Receiver<()>,
}

/// This machine's door for a browser. Cloning shares the one door.
#[derive(Clone)]
pub struct LocalDoor(Arc<Inner>);

struct Inner {
    held: Mutex<Held>,
    senses: DoorSenses,
    keep: KeepBrowsers,
    code: watch::Sender<String>,
    /// Sent whenever a session token may have ended while a socket held
    /// it: a device forgotten, or the table full.
    ended: watch::Sender<()>,
}

struct Held {
    door: BrowserDoor,
    waiting: BTreeMap<B3Hash, mpsc::Sender<()>>,
}

impl LocalDoor {
    /// A door that knows `browsers`, showing a fresh pairing code.
    ///
    /// # Errors
    /// The random source refuses.
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

    /// The pairing code shown now, updated every time a guess replaces it.
    #[must_use]
    pub fn pairing_code(&self) -> watch::Receiver<String> {
        self.0.code.subscribe()
    }

    /// Mints an open code for `/web` to hand over.
    ///
    /// # Errors
    /// The clock or the random source refuses.
    pub fn issue_open_code(&self) -> Result<OpenCode, AxError> {
        let entropy: [u8; 16] = drawn(&self.0.senses)?;
        let now = (self.0.senses.clock)()?;
        let (tell, redeemed) = mpsc::channel();
        let mut held = self.held();
        let code = held.door.issue_open_code(entropy, now);
        held.waiting.insert(B3Hash::digest(code.as_bytes()), tell);
        Ok(OpenCode { code, redeemed })
    }

    /// Every paired browser.
    #[must_use]
    pub fn devices(&self) -> DevicesAnswer {
        self.held().door.devices()
    }

    /// Forgets a paired browser and ends its sessions, closing every
    /// socket that holds one of them; false when it was not paired.
    ///
    /// # Errors
    /// The table could not be kept; the browser stays paired.
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

    /// Runs `judge` over the session tokens and the moment they are
    /// judged at.
    ///
    /// # Errors
    /// The clock refuses; nothing is judged.
    pub(crate) fn with_sessions<R>(
        &self,
        judge: impl FnOnce(&Sessions, TimeMs) -> R,
    ) -> Result<R, AxError> {
        let now = (self.0.senses.clock)()?;
        Ok(judge(self.held().door.sessions(), now))
    }

    /// Whether `offered` is the key `face` demands or a session token
    /// that admits now.
    ///
    /// # Errors
    /// The clock refuses.
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

    /// Seats a socket greeted with `presented`: a session token that
    /// admits now is held for as long as the seat lives, so it does not
    /// lapse under an open socket; the native key takes no seat.
    ///
    /// The seat starts marked as ended, so the socket judges its
    /// credential once more at once: a token ended between the hello's
    /// judgement and this seat ends the socket as well.
    ///
    /// # Errors
    /// The clock refuses.
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

    /// Pairs a browser that presented one of the two codes.
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
                    // A send that finds nobody means the opener stopped
                    // waiting, and its file went with it: nothing is owed.
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

    /// One socket seated on `token` has ended; the last one to leave
    /// starts the token's idle allowance now.
    pub(super) fn unseat(&self, token: &B3Hash) {
        let now = (self.0.senses.clock)();
        let mut held = self.held();
        match now {
            Ok(now) => held.door.sessions_mut().unseat(token, now),
            // Without a time the idle allowance cannot start, so the
            // token ends now; the page signs a new one before it dials.
            Err(_) => held.door.sessions_mut().end(token),
        }
    }

    /// A nonce for a paired browser to sign.
    pub(crate) fn challenge(&self) -> Result<String, AxError> {
        let entropy: [u8; 32] = drawn(&self.0.senses)?;
        let now = (self.0.senses.clock)()?;
        Ok(self.held().door.challenge(entropy, now))
    }

    /// Opens a session for a paired browser whose signature over
    /// `nonce`, `origin` and the label verifies.
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
        // A full table ends its oldest token, which a socket may hold.
        self.0.ended.send_replace(());
        Ok(token)
    }

    /// The state, even after a holder panicked: every transition leaves it
    /// whole, so a poisoned lock holds nothing half-written.
    fn held(&self) -> MutexGuard<'_, Held> {
        self.0.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `N` bytes from the door's random source.
fn drawn<const N: usize>(senses: &DoorSenses) -> Result<[u8; N], AxError> {
    let mut bytes = [0u8; N];
    (senses.entropy)(&mut bytes)?;
    Ok(bytes)
}

/// A label a person can read: not empty, no control characters, at most
/// [`LABEL_MAX`] characters.
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
