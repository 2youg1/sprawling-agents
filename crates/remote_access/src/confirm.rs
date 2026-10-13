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

/// The least time between two answers the guard reads
/// (`crates/remote_access/Spec.lean` D25): one a second, the rate the
/// pairing code is guessed at.
pub const CONFIRM_SPACING_MS: u64 = 1_000;

/// How many read answers may miss, since the last one that did its verb,
/// before the guard refuses every request and every answer until the city
/// restarts (D25). A client that never read the console therefore wins
/// with chance at most this many in 2^40.
pub const CONFIRM_MISSES_MAX: u32 = 10;

/// The request that waits, if any, for the verb type `V` the caller
/// carries out once it is confirmed, behind the guard that spaces and
/// counts the answers (D25).
#[derive(Debug)]
pub struct Confirm<V> {
    pending: Option<Pending<V>>,
    /// When the last read answer came; `None` before the first.
    last: Option<TimeMs>,
    /// Read answers that missed since the last one that did its verb.
    misses: u32,
}

#[derive(Debug)]
struct Pending<V> {
    verb: V,
    code: B3Hash,
    expires: TimeMs,
}

/// Why an answer did no verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Miss {
    /// Read and refused: no request waits, the answer is not its code, or
    /// it came too late. The request is gone.
    Wrong,
    /// Not read: it came within [`CONFIRM_SPACING_MS`] of the last answer
    /// that was. The request still waits.
    TooSoon,
    /// Not read: [`CONFIRM_MISSES_MAX`] answers missed since the last one
    /// that did its verb, and the guard holds until the city restarts.
    Locked,
}

impl Miss {
    /// What the page that sent the answer reads.
    #[must_use]
    pub fn refusal(self) -> AxError {
        let (subject, recovery) = match self {
            Miss::Wrong => (
                "no code is waiting, the code is not the one the city's console printed, or it expired",
                "press the control on the page again, then type the new code the city's console prints",
            ),
            Miss::TooSoon => (
                "this answer came within a second of the last one, so it was not read",
                "wait a second, then type the same code again",
            ),
            Miss::Locked => (
                "ten wrong codes were typed since the last right one, so the page's remote-door \
                 controls are off until the city restarts",
                "use /remote at the city's console, the terminal running `sprawling serve`",
            ),
        };
        AxError::failure(AxCode::GateDenied, "confirm a remote door verb", subject)
            .with_recovery(recovery)
    }
}

impl<V> Default for Confirm<V> {
    fn default() -> Self {
        Self {
            pending: None,
            last: None,
            misses: 0,
        }
    }
}

impl<V> Confirm<V> {
    /// Waits for `verb`, replacing any request before it, and returns the
    /// code to print at the console, grouped for reading (`abcd-efgh`).
    /// Nothing here keeps the text.
    ///
    /// # Errors
    /// `E_GATE_DENIED` once the guard is locked: a code nobody may answer
    /// is not printed.
    pub fn request(
        &mut self,
        verb: V,
        entropy: [u8; CONFIRM_BYTES],
        expires: TimeMs,
    ) -> Result<String, AxError> {
        if self.locked() {
            return Err(Miss::Locked.refusal());
        }
        let canonical = encode(&entropy);
        self.pending = Some(Pending {
            verb,
            code: B3Hash::digest(canonical.as_bytes()),
            expires,
        });
        Ok(canonical
            .as_bytes()
            .chunks(GROUP_LEN)
            .map(|group| {
                group
                    .iter()
                    .map(|byte| char::from(*byte))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("-"))
    }

    /// Answers the request that waits with `answer`, read the way it was
    /// printed (case, spaces and dashes do not matter).
    ///
    /// # Errors
    /// [`Miss::Locked`] and [`Miss::TooSoon`] for an answer the guard does
    /// not read; [`Miss::Wrong`] for one it reads that does no verb, which
    /// ends the request and counts toward [`CONFIRM_MISSES_MAX`].
    pub fn confirm(&mut self, answer: &str, now: TimeMs) -> Result<V, Miss> {
        if self.locked() {
            return Err(Miss::Locked);
        }
        let spaced = self
            .last
            .is_none_or(|last| last.value().saturating_add(CONFIRM_SPACING_MS) <= now.value());
        if !spaced {
            return Err(Miss::TooSoon);
        }
        self.last = Some(now);
        let typed = B3Hash::digest(canonical(answer).as_bytes());
        match self.pending.take() {
            Some(pending) if pending.code == typed && now < pending.expires => {
                self.misses = 0;
                Ok(pending.verb)
            }
            Some(_) | None => {
                self.misses = self.misses.saturating_add(1);
                Err(Miss::Wrong)
            }
        }
    }

    /// Whether the guard refuses every request and answer (D25).
    #[must_use]
    pub fn locked(&self) -> bool {
        self.misses >= CONFIRM_MISSES_MAX
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    //! The trace vectors of `spec/Confirm.lean`: its `guarded`, with the
    //! `step` it wraps, transcribed as the reference with this module's
    //! two constants, against `Confirm` on every trace of up to four
    //! events over an alphabet spread across the spacing, so each of the
    //! model's five properties is judged on the Rust machine.

    use super::*;

    const SPACING: u64 = CONFIRM_SPACING_MS;
    const TTL: u64 = 2 * SPACING;

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
    enum Said {
        Shown(u8),
        Done(Guarded),
        Missed,
        TooSoon,
        LockedOut,
    }

    /// The pending request, the last read answer, the misses since the
    /// last verb done.
    type State = (Option<(Guarded, u8, u64)>, Option<u64>, u32);

    /// `RemoteConfirm.guarded`, word for word.
    fn guarded(s: State, e: Event) -> (State, Said) {
        let (pending, last, misses) = s;
        if CONFIRM_MISSES_MAX <= misses {
            return (s, Said::LockedOut);
        }
        match e {
            Event::Request(v, a, now) => ((Some((v, a, now + TTL)), last, misses), Said::Shown(a)),
            Event::Confirm(a, now) => {
                if !last.is_none_or(|t| t + SPACING <= now) {
                    return (s, Said::TooSoon);
                }
                match pending {
                    Some((v, secret, expires)) if a == secret && now < expires => {
                        ((None, Some(now), 0), Said::Done(v))
                    }
                    Some(_) | None => ((None, Some(now), misses + 1), Said::Missed),
                }
            }
        }
    }

    fn model(trace: &[Event]) -> Vec<Said> {
        let mut s = (None, None, 0);
        trace
            .iter()
            .map(|e| {
                let (next, said) = guarded(s, *e);
                s = next;
                said
            })
            .collect()
    }

    fn code(secret: u8) -> String {
        Confirm::<()>::default()
            .request((), [secret; CONFIRM_BYTES], TimeMs::new(0))
            .unwrap()
    }

    fn played(confirm: &mut Confirm<Guarded>, trace: &[Event]) -> Vec<Said> {
        trace
            .iter()
            .map(|e| match *e {
                Event::Request(v, a, now) => {
                    match confirm.request(v, [a; CONFIRM_BYTES], TimeMs::new(now + TTL)) {
                        Ok(shown) => {
                            assert_eq!(shown, code(a));
                            Said::Shown(a)
                        }
                        Err(refused) => {
                            assert_eq!(refused.code(), &AxCode::GateDenied);
                            Said::LockedOut
                        }
                    }
                }
                Event::Confirm(a, now) => {
                    let typed = code(a).to_uppercase().replace('-', " ");
                    match confirm.confirm(&typed, TimeMs::new(now)) {
                        Ok(v) => Said::Done(v),
                        Err(Miss::Wrong) => Said::Missed,
                        Err(Miss::TooSoon) => Said::TooSoon,
                        Err(Miss::Locked) => Said::LockedOut,
                    }
                }
            })
            .collect()
    }

    fn machine(trace: &[Event]) -> Vec<Said> {
        played(&mut Confirm::default(), trace)
    }

    fn alphabet() -> Vec<Event> {
        let mut events = Vec::new();
        for v in [Guarded::OpenDoor, Guarded::ReplaceKey] {
            for a in [1, 2] {
                for t in [0, 3 * SPACING] {
                    events.push(Event::Request(v, a, t));
                }
            }
        }
        for a in [1, 2, 3] {
            for now in [SPACING / 2, 3 * SPACING / 2, 7 * SPACING / 2] {
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

    /// The first three properties, one vector each: a wrong guess ends
    /// the request, an expired code does nothing, an answer in time does
    /// the verb.
    #[test]
    fn one_guess_per_request_and_none_after_expiry() {
        let o = Guarded::OpenDoor;
        assert_eq!(
            [
                machine(&[
                    Event::Request(o, 1, 0),
                    Event::Confirm(2, SPACING),
                    Event::Confirm(1, 2 * SPACING)
                ]),
                machine(&[Event::Request(o, 1, 0), Event::Confirm(1, TTL)]),
                machine(&[Event::Request(o, 1, 0), Event::Confirm(1, SPACING)]),
            ],
            [
                vec![Said::Shown(1), Said::Missed, Said::Missed],
                vec![Said::Shown(1), Said::Missed],
                vec![Said::Shown(1), Said::Done(o)],
            ]
        );
    }

    /// An answer within a second of the last read one is not read, and
    /// the request it answers still waits for the same code.
    #[test]
    fn an_answer_inside_the_spacing_is_not_read_and_the_request_waits() {
        let o = Guarded::OpenDoor;
        let trace = [
            Event::Request(o, 1, 0),
            Event::Confirm(2, SPACING / 2),
            Event::Request(o, 3, SPACING / 2),
            Event::Confirm(3, SPACING),
            Event::Confirm(3, 3 * SPACING / 2),
        ];
        assert_eq!(
            machine(&trace),
            [
                Said::Shown(1),
                Said::Missed,
                Said::Shown(3),
                Said::TooSoon,
                Said::Done(o)
            ]
        );
    }

    /// Ten read misses lock the guard: the next request prints no code
    /// and the next answer is not read, the right one included. A verb
    /// done before the tenth miss clears the count.
    #[test]
    fn ten_misses_lock_every_request_and_answer_and_a_verb_done_clears_the_count() {
        let o = Guarded::OpenDoor;
        let missing = |from: u32, count: u32| -> Vec<Event> {
            (from..from + count)
                .flat_map(|k| {
                    let at = u64::from(k) * SPACING;
                    [Event::Request(o, 1, at), Event::Confirm(2, at)]
                })
                .collect()
        };
        let mut locking = Confirm::default();
        let mut tail = played(&mut locking, &missing(0, CONFIRM_MISSES_MAX));
        let after = u64::from(CONFIRM_MISSES_MAX) * SPACING;
        tail.extend(played(
            &mut locking,
            &[Event::Request(o, 1, after), Event::Confirm(1, after)],
        ));

        let mut clearing = Confirm::default();
        let nine = CONFIRM_MISSES_MAX - 1;
        played(&mut clearing, &missing(0, nine));
        let at = u64::from(nine) * SPACING;
        let done = played(
            &mut clearing,
            &[Event::Request(o, 1, at), Event::Confirm(1, at)],
        );
        played(&mut clearing, &missing(nine + 1, nine));

        assert_eq!(
            (
                tail.split_off(tail.len() - 3),
                locking.locked(),
                done,
                clearing.locked()
            ),
            (
                vec![Said::Missed, Said::LockedOut, Said::LockedOut],
                true,
                vec![Said::Shown(1), Said::Done(o)],
                false
            )
        );
    }

    #[test]
    fn the_code_is_eight_base32_symbols_in_two_groups() {
        let shown = code(0xff);
        assert_eq!(shown, "7777-7777");
    }
}
