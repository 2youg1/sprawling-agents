// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The live session tokens of this machine's door for a browser, as a
//! state machine (`crates/wire/spec/Server/Sessions.lean` §8-93s): whose each token
//! is, how many sockets hold it, and when one that no socket holds
//! lapses.
//!
//! Pure: every transition is handed its time. A token is judged at the
//! moment it is asked about, so a lapsed token needs no sweep to stop
//! admitting; minting a new one clears the lapsed ones so the table does
//! not grow.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use kernel::{B3Hash, TimeMs};

use crate::answer::DeviceId;

/// Live session tokens at once; the oldest idle one goes first.
pub const SESSIONS_MAX: usize = 64;
/// How long a token no socket holds stays good (wire D55).
pub const SESSION_IDLE_MS: u64 = 60_000;

/// Whether sockets hold a token now, or since when none has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hold {
    Seated(NonZeroUsize),
    Idle(TimeMs),
}

/// One token: the device that signed for it, its place in the order of
/// minting, and who holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Session {
    device: DeviceId,
    minted: u64,
    hold: Hold,
}

impl Session {
    /// Whether no socket holds the token.
    fn idle(&self) -> bool {
        match self.hold {
            Hold::Seated(_) => false,
            Hold::Idle(_) => true,
        }
    }

    /// Whether the token still admits at `now`.
    fn live(&self, now: TimeMs) -> bool {
        match self.hold {
            Hold::Seated(_) => true,
            Hold::Idle(since) => now.value() < since.value().saturating_add(SESSION_IDLE_MS),
        }
    }
}

/// The live session tokens, by digest, each belonging to one device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sessions {
    live: BTreeMap<B3Hash, Session>,
    minted: u64,
}

impl Sessions {
    /// Whether `presented` is a session token that admits at `now`.
    #[must_use]
    pub fn holds(&self, presented: &str, now: TimeMs) -> bool {
        self.live
            .get(&B3Hash::digest(presented.as_bytes()))
            .is_some_and(|session| session.live(now))
    }

    /// Keeps `token` for `device`, idle from `now`, after clearing the
    /// lapsed tokens and, while the table is full, the oldest idle one -
    /// or the oldest of all when sockets hold every one.
    pub(super) fn mint(&mut self, device: DeviceId, token: &str, now: TimeMs) {
        self.live.retain(|_, session| session.live(now));
        while self.live.len() >= SESSIONS_MAX {
            let oldest = |idle_only: bool| {
                self.live
                    .iter()
                    .filter(|(_, session)| !idle_only || session.idle())
                    .min_by_key(|(_, session)| session.minted)
                    .map(|(digest, _)| *digest)
            };
            let Some(evicted) = oldest(true).or_else(|| oldest(false)) else {
                break;
            };
            self.live.remove(&evicted);
        }
        self.minted = self.minted.saturating_add(1);
        self.live.insert(
            B3Hash::digest(token.as_bytes()),
            Session {
                device,
                minted: self.minted,
                hold: Hold::Idle(now),
            },
        );
    }

    /// Seats one socket on `presented` when it is a token that admits at
    /// `now`, and names the token so the socket can leave its seat; `None`
    /// for anything else, the native key included.
    pub(crate) fn seat(&mut self, presented: &str, now: TimeMs) -> Option<B3Hash> {
        let digest = B3Hash::digest(presented.as_bytes());
        let session = self
            .live
            .get_mut(&digest)
            .filter(|session| session.live(now))?;
        session.hold = Hold::Seated(match session.hold {
            Hold::Seated(sockets) => sockets.saturating_add(1),
            Hold::Idle(_) => NonZeroUsize::MIN,
        });
        Some(digest)
    }

    /// One socket seated on `digest` has ended; the last one to leave
    /// starts the idle allowance at `now`.
    pub(crate) fn unseat(&mut self, digest: &B3Hash, now: TimeMs) {
        if let Some(session) = self.live.get_mut(digest)
            && let Hold::Seated(sockets) = session.hold
        {
            session.hold = NonZeroUsize::new(sockets.get().saturating_sub(1))
                .map_or(Hold::Idle(now), Hold::Seated);
        }
    }

    /// Ends the token `digest` at once, whoever holds it.
    pub(crate) fn end(&mut self, digest: &B3Hash) {
        self.live.remove(digest);
    }

    /// Ends every token `device` signed for.
    pub(super) fn end_device(&mut self, device: &DeviceId) {
        self.live.retain(|_, session| &session.device != device);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use proptest::prelude::*;

    use super::*;
    use crate::reception::pairing::{BrowserDoor, DeviceKey, PairedBrowser};

    /// One step of the model's trace (`Wire.Server.Sessions.Step`), with
    /// the clock moving between steps. Devices and tokens are picked by
    /// index, so a trace keeps meeting the ones it already made.
    #[derive(Debug, Clone)]
    enum Move {
        Pair(u8),
        Mint(u8),
        Seat(usize),
        Unseat(usize),
        Forget(u8),
        Wait(u64),
    }

    fn a_move() -> impl Strategy<Value = Move> {
        prop_oneof![
            (0u8..3).prop_map(Move::Pair),
            (0u8..3).prop_map(Move::Mint),
            (0usize..8).prop_map(Move::Seat),
            (0usize..8).prop_map(Move::Unseat),
            (0u8..3).prop_map(Move::Forget),
            (0u64..2 * SESSION_IDLE_MS).prop_map(Move::Wait),
        ]
    }

    /// What the trace did to one token, as the three properties read it.
    struct Minted {
        token: String,
        device: DeviceId,
        seats: usize,
        idle_since: u64,
        lapsed: bool,
    }

    proptest! {
        /// `crates/wire/spec/Server/Sessions.lean` §8-93s over random traces that
        /// never fill the table: a token no socket holds admits for
        /// `idle` and no longer (`Session.live`); a forgotten device that is not paired
        /// again has no token that admits (`a_forgotten_device_keeps_no_session`);
        /// a token that has stopped admitting never admits again
        /// (`an_expired_token_never_admits_again`); a token a socket
        /// holds admits however long passes (`a_seated_token_outlasts_its_idle_time`).
        #[test]
        fn forgotten_and_lapsed_tokens_stay_ended_and_seated_ones_stay_good(
            moves in proptest::collection::vec(a_move(), 0..48),
        ) {
            let mut door = BrowserDoor::new(Vec::new(), [0; 32]);
            let mut now = 0u64;
            let mut minted: Vec<Minted> = Vec::new();
            let mut devices: BTreeMap<u8, DeviceId> = BTreeMap::new();
            let mut forgotten: BTreeSet<DeviceId> = BTreeSet::new();
            for (step, each) in moves.into_iter().enumerate() {
                let at = TimeMs::new(now);
                match each {
                    Move::Pair(which) => {
                        let line = PairedBrowser::new(DeviceKey::parse(&format!("{which:02x}").repeat(32)).unwrap(), "browser".to_owned(), at);
                        let device = door.pair(line);
                        forgotten.remove(&device);
                        devices.insert(which, device);
                    }
                    Move::Mint(which) => {
                        let Some(device) = devices.get(&which) else { continue };
                        let entropy = [u8::try_from(step).unwrap(); 32];
                        if let Some(token) = door.open_session(device, entropy, at) {
                            minted.push(Minted { token, device: device.clone(), seats: 0, idle_since: now, lapsed: false });
                        }
                    }
                    Move::Seat(pick) => {
                        let Some(token) = minted.get_mut(pick) else { continue };
                        if door.sessions_mut().seat(&token.token, at).is_some() {
                            token.seats += 1;
                        }
                    }
                    Move::Unseat(pick) => {
                        let Some(token) = minted.get_mut(pick) else { continue };
                        if token.seats > 0 {
                            door.sessions_mut().unseat(&B3Hash::digest(token.token.as_bytes()), at);
                            token.seats -= 1;
                            token.idle_since = now;
                        }
                    }
                    Move::Forget(which) => {
                        let Some(device) = devices.get(&which) else { continue };
                        door.forget(device);
                        forgotten.insert(device.clone());
                        for token in minted.iter_mut().filter(|token| &token.device == device) {
                            token.seats = 0;
                        }
                    }
                    Move::Wait(ms) => now += ms,
                }
                let at = TimeMs::new(now);
                for token in &mut minted {
                    let admits = door.sessions().holds(&token.token, at);
                    if forgotten.contains(&token.device) {
                        prop_assert!(!admits, "a token of a forgotten device admits");
                    }
                    if token.lapsed {
                        prop_assert!(!admits, "a token admits again after it stopped");
                    }
                    if token.seats > 0 {
                        prop_assert!(admits, "a token a socket holds stopped admitting");
                    } else if now >= token.idle_since + SESSION_IDLE_MS {
                        prop_assert!(!admits, "an idle token outlived its allowance");
                    }
                    token.lapsed = !admits;
                }
            }
        }
    }
}
