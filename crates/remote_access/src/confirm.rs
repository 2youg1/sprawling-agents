// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one request that waits for the city's console: a page asks for a
//! verb, the console shows a code, the User types it back
//! (`crates/remote_access/spec/Confirm.lean`, remote_access D4).
//!
//! At most one request waits. A new request replaces the one before it,
//! and every answer, right or wrong, ends the request it answers, so one
//! request gives one guess. The code is kept as the digest of its
//! canonical text, for the reason the pairing code is (§8-2). Entropy and
//! the expiry arrive as parameters, so the assembly layer draws and
//! samples them.

use kernel::{AxCode, AxError, B3Hash, TimeMs};

use crate::pairing::{canonical, encode};

/// The bytes of entropy one confirmation code is drawn from: 40 bits,
/// eight base32 symbols. One request gives one guess before it ends, so
/// a guesser's chance is one in 2^40 per request the User sees printed.
pub const CONFIRM_BYTES: usize = 5;

/// Symbols per group in the form the console prints.
const GROUP_LEN: usize = 4;

/// The request that waits, if any, for the verb type `V` the caller
/// carries out once it is confirmed.
#[derive(Debug)]
pub struct Confirm<V> {
    pending: Option<Pending<V>>,
}

#[derive(Debug)]
struct Pending<V> {
    verb: V,
    code: B3Hash,
    expires: TimeMs,
}

impl<V> Default for Confirm<V> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<V> Confirm<V> {
    /// Waits for `verb`, replacing any request before it, and returns the
    /// code to print at the console, grouped for reading (`abcd-efgh`).
    /// Nothing here keeps the text.
    pub fn request(&mut self, verb: V, entropy: [u8; CONFIRM_BYTES], expires: TimeMs) -> String {
        let canonical = encode(&entropy);
        self.pending = Some(Pending {
            verb,
            code: B3Hash::digest(canonical.as_bytes()),
            expires,
        });
        canonical
            .as_bytes()
            .chunks(GROUP_LEN)
            .map(|group| {
                group
                    .iter()
                    .map(|byte| char::from(*byte))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Answers the request that waits with `answer`, read the way it was
    /// printed (case, spaces and dashes do not matter), and ends it.
    ///
    /// # Errors
    /// `E_GATE_DENIED` when no request waits, the answer is not its code,
    /// or `now` is not before its expiry; the request is gone either way.
    pub fn confirm(&mut self, answer: &str, now: TimeMs) -> Result<V, AxError> {
        let typed = B3Hash::digest(canonical(answer).as_bytes());
        match self.pending.take() {
            Some(pending) if pending.code == typed && now < pending.expires => Ok(pending.verb),
            Some(pending) if pending.code != typed => {
                self.pending = Some(pending);
                Err(denied())
            }
            Some(_) | None => Err(denied()),
        }
    }
}

fn denied() -> AxError {
    AxError::failure(
        AxCode::GateDenied,
        "confirm a remote door verb",
        "no code is waiting, the code is not the one the city's console printed, or it expired",
    )
    .with_recovery(
        "press the control on the page again, then type the new code the city's console prints",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    //! The trace vectors of `spec/Confirm.lean`: its `step`, transcribed
    //! as the reference, against `Confirm` on every trace of up to four
    //! events over a small alphabet, so each of the model's three
    //! properties is judged on the Rust machine.

    use super::*;

    const TTL: u64 = 2;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Guarded {
        OpenDoor,
        ReplaceKey,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Request(Guarded, u8, u64),
        Confirm(u8, u64),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Outcome {
        Shown(u8),
        Done(Guarded),
        Refused,
    }

    /// `RemoteConfirm.step`, word for word.
    fn step(s: Option<(Guarded, u8, u64)>, e: Event) -> (Option<(Guarded, u8, u64)>, Outcome) {
        match (e, s) {
            (Event::Request(v, a, now), _) => (Some((v, a, now + TTL)), Outcome::Shown(a)),
            (Event::Confirm(a, now), Some((v, secret, expires))) => {
                if a == secret && now < expires {
                    (None, Outcome::Done(v))
                } else {
                    (None, Outcome::Refused)
                }
            }
            (Event::Confirm(..), None) => (None, Outcome::Refused),
        }
    }

    fn model(trace: &[Event]) -> Vec<Outcome> {
        let mut s = None;
        trace
            .iter()
            .map(|e| {
                let (next, out) = step(s, *e);
                s = next;
                out
            })
            .collect()
    }

    fn code(secret: u8) -> String {
        Confirm::<()>::default().request((), [secret; CONFIRM_BYTES], TimeMs::new(0))
    }

    fn machine(trace: &[Event]) -> Vec<Outcome> {
        let mut confirm = Confirm::default();
        trace
            .iter()
            .map(|e| match *e {
                Event::Request(v, a, now) => {
                    let shown = confirm.request(v, [a; CONFIRM_BYTES], TimeMs::new(now + TTL));
                    assert_eq!(shown, code(a));
                    Outcome::Shown(a)
                }
                Event::Confirm(a, now) => {
                    match confirm
                        .confirm(&code(a).to_uppercase().replace('-', " "), TimeMs::new(now))
                    {
                        Ok(v) => Outcome::Done(v),
                        Err(refused) => {
                            assert_eq!(refused.code(), &AxCode::GateDenied);
                            Outcome::Refused
                        }
                    }
                }
            })
            .collect()
    }

    fn alphabet() -> Vec<Event> {
        let mut events = Vec::new();
        for v in [Guarded::OpenDoor, Guarded::ReplaceKey] {
            for a in [1, 2] {
                for t in [0, 2] {
                    events.push(Event::Request(v, a, t));
                }
            }
        }
        for a in [1, 2, 3] {
            for now in [1, 2, 4] {
                events.push(Event::Confirm(a, now));
            }
        }
        events
    }

    #[test]
    fn every_short_trace_does_what_the_model_does() {
        let alphabet = alphabet();
        let mut traces: Vec<Vec<Event>> = vec![Vec::new()];
        let mut checked = 0_usize;
        for _ in 0..4 {
            traces = traces
                .iter()
                .flat_map(|trace| {
                    alphabet.iter().map(move |e| {
                        let mut longer = trace.clone();
                        longer.push(*e);
                        longer
                    })
                })
                .collect();
            for trace in &traces {
                assert_eq!(machine(trace), model(trace), "{trace:?}");
                checked += 1;
            }
        }
        assert!(checked > 80_000, "{checked}");
    }

    /// The three named properties, one vector each: a wrong guess ends
    /// the request, an expired code does nothing, an answer in time
    /// does the verb.
    #[test]
    fn one_guess_per_request_and_none_after_expiry() {
        let o = Guarded::OpenDoor;
        assert_eq!(
            [
                machine(&[
                    Event::Request(o, 1, 0),
                    Event::Confirm(2, 1),
                    Event::Confirm(1, 1)
                ]),
                machine(&[Event::Request(o, 1, 0), Event::Confirm(1, TTL)]),
                machine(&[Event::Request(o, 1, 0), Event::Confirm(1, 1)]),
            ],
            [
                vec![Outcome::Shown(1), Outcome::Refused, Outcome::Refused],
                vec![Outcome::Shown(1), Outcome::Refused],
                vec![Outcome::Shown(1), Outcome::Done(o)],
            ]
        );
    }

    #[test]
    fn the_code_is_eight_base32_symbols_in_two_groups() {
        let shown = code(0xff);
        assert_eq!(shown, "7777-7777");
    }
}
