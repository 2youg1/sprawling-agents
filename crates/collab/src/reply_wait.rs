// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The waiting send (collab D9; `spec/Delivery.lean` §6, `wait_bounded`
//! and `waits_end`): a run that sends with `wait` stops at its next
//! safe point, makes no model call, and goes on when the room it spoke
//! to replies, when the injected clock reaches the deadline, or when
//! it leaves its room.
//!
//! The deadline is the reason two runs that wait on each other need no
//! deadlock detection: every wait ends at most [`PATIENCE_MS`] after it
//! started. A reply is any signal from the room waited on that reaches
//! the waiting run's slot — the same door a steer comes through — so
//! the wait reads no second queue. Time is a parameter and order is the
//! slot's, so a trace gives one result on Windows, macOS and Linux.

use kernel::event::record::{SignalId, SignalWaitEnded, SignalWaitStarted, WaitEnd};
use kernel::{Address, AxCode, AxError, TimeMs};
use serde_json::{Map, Value};

use crate::inbox::Signal;
use crate::signal_desk::{SignalDesk, SignalEffect};
use crate::signal_tool::PATIENCE_MS;

/// One run's wait for a reply: whom it waits on, which send asked for
/// it, and — once the run reached its safe point — the reading of the
/// injected clock that ends it. The deadline is taken at the stop, not
/// at the send, as `Collab.Delivery.step`'s `park` takes `clock`.
/// `kept` is a reply that arrived after the send and before the stop,
/// held here rather than queued (collab D15, `spec/Delivery.lean` §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplyWait {
    on: Address,
    signal: SignalId,
    deadline: Option<TimeMs>,
    kept: Option<Signal>,
}

impl ReplyWait {
    pub(crate) fn asked(on: Address, signal: SignalId) -> ReplyWait {
        ReplyWait {
            on,
            signal,
            deadline: None,
            kept: None,
        }
    }

    /// Keeps `signal` for this wait when it is the first reply to arrive
    /// (`Collab.Delivery.Early.collect`); hands it back otherwise, for
    /// the queue.
    pub(crate) fn keep(&mut self, signal: Signal, room: &Address) -> Option<Signal> {
        if self.kept.is_none() && self.answers(&signal, room) {
            self.kept = Some(signal);
            return None;
        }
        Some(signal)
    }

    /// The reply kept for this wait, taken out of it: what a run that
    /// leaves before its stop puts back in the queue.
    pub(crate) fn unkeep(&mut self) -> Option<Signal> {
        self.kept.take()
    }

    fn answers(&self, signal: &Signal, room: &Address) -> bool {
        signal.from() == self.on.as_str() && signal.room() == room
    }

    /// The deadline this wait ends at, fixed at its first safe point,
    /// and the `signal_wait_started` line when that is now.
    fn park(&mut self, now: TimeMs) -> (TimeMs, Option<SignalWaitStarted>) {
        if let Some(deadline) = self.deadline {
            return (deadline, None);
        }
        let deadline = TimeMs::new(now.value().saturating_add(PATIENCE_MS));
        self.deadline = Some(deadline);
        (
            deadline,
            Some(SignalWaitStarted {
                on: self.on.clone(),
                signal: self.signal.clone(),
                deadline_ms: deadline.value(),
            }),
        )
    }

    /// What the send's result tells the model about the stop it is
    /// about to make.
    pub(crate) fn promise(&self) -> String {
        format!(
            "you stop at your next step without a model call until {} replies or {} s pass",
            self.on.as_str(),
            PATIENCE_MS / 1000
        )
    }

    fn ended(&self, by: WaitEnd) -> SignalEffect {
        SignalEffect::WaitEnded(SignalWaitEnded {
            signal: self.signal.clone(),
            by,
        })
    }
}

/// Reads the send's `wait` input: absent means the asynchronous
/// default. A run waits for one reply at a time, so a second waiting
/// send before the first wait ended is refused.
///
/// # Errors
/// `E_INVALID_ARGS` for a value that is not a yes or a no, or for a
/// second wait.
pub(crate) fn wanted(args: &Map<String, Value>, already: bool) -> Result<bool, AxError> {
    let wants = match args.get("wait") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(wants)) => *wants,
        Some(Value::String(raw)) if raw == "true" => true,
        Some(Value::String(raw)) if raw == "false" => false,
        Some(other) => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "send a signal",
                format!("`wait` is {other}"),
            )
            .with_recovery("pass `wait` as true or false, or leave it out"));
        }
    };
    if wants && already {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "send a signal",
            "this run already waits for a reply",
        )
        .with_recovery("send without `wait`; the wait you started ends on its own"));
    }
    Ok(wants)
}

/// What a safe point learns from a run's wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitTurn {
    /// The run is not waiting.
    Idle,
    /// Still waiting: ask again after a slice of the clock.
    Waiting,
    /// The room waited on replied: its words land as a letter of kind
    /// `reply` (collab D16).
    Replied(crate::Letter),
    /// Nobody replied in time; `text` is the city's own word, and lands
    /// as the city's.
    TimedOut { text: String },
}

impl SignalDesk {
    /// Advances this run's wait to `now`: a reply in the slot ends it,
    /// then the deadline does. The reply is held like a steer, so it is
    /// consumed once the answer that read it lands (collab D8); every
    /// other signal in the slot goes into the lent queue.
    ///
    /// # Errors
    /// Propagates a slot, a queue or a post that fails; the wait is kept
    /// so the next safe point asks again.
    pub fn wait_out(&mut self, now: TimeMs) -> Result<WaitTurn, AxError> {
        let Some(mut wait) = self.waiting.clone() else {
            return Ok(WaitTurn::Idle);
        };
        let (deadline, started) = wait.park(now);
        if let Some(started) = started {
            (self.post.0)(&SignalEffect::WaitStarted(started))?;
            self.waiting = Some(wait.clone());
        }
        let mut reply = wait.unkeep();
        for signal in self.slot.take()? {
            if reply.is_none() && wait.answers(&signal, &self.room) {
                reply = Some(signal);
            } else {
                self.admit(&signal)?;
            }
        }
        if let Some(reply) = reply {
            (self.post.0)(&wait.ended(WaitEnd::Reply {
                reply: reply.id().clone(),
            }))?;
            self.waiting = None;
            let letter = crate::Letter::reply(&reply);
            self.held.push(reply);
            return Ok(WaitTurn::Replied(letter));
        }
        if now < deadline {
            return Ok(WaitTurn::Waiting);
        }
        (self.post.0)(&wait.ended(WaitEnd::Timeout))?;
        self.waiting = None;
        Ok(WaitTurn::TimedOut {
            text: format!(
                "no reply came from {} within {} s; go on without it",
                wait.on.as_str(),
                PATIENCE_MS / 1000
            ),
        })
    }

    /// Ends this run's wait because the run is leaving its room
    /// (`Collab.Delivery.City.vacate`); nothing when it was not waiting,
    /// and no line when it left before its first safe point, because no
    /// `signal_wait_started` was written for it. A reply kept for the
    /// wait goes back to the front of the queue (`Collab.Delivery.Early.leave`).
    ///
    /// # Errors
    /// Propagates a post that fails; the wait is then still recorded as
    /// open, which the ledger reader reads as a wait the run left.
    pub fn wait_left(&mut self) -> Result<(), AxError> {
        let Some(mut wait) = self.waiting.take() else {
            return Ok(());
        };
        if let Some(kept) = wait.unkeep() {
            self.inbox.give_back(vec![kept]);
        }
        match wait.deadline {
            Some(_) => (self.post.0)(&wait.ended(WaitEnd::Left)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests;
