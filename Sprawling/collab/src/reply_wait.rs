// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{SignalId, SignalWaitEnded, SignalWaitStarted, WaitEnd};
use kernel::{Address, AxCode, AxError, TimeMs};
use serde_json::{Map, Value};

use crate::inbox::Signal;
use crate::signal_desk::{SignalDesk, SignalEffect};
use crate::signal_tool::PATIENCE_MS;

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

    pub(crate) fn keep(&mut self, signal: Signal, room: &Address) -> Option<Signal> {
        if self.kept.is_none() && self.answers(&signal, room) {
            self.kept = Some(signal);
            return None;
        }
        Some(signal)
    }

    pub(crate) fn unkeep(&mut self) -> Option<Signal> {
        self.kept.take()
    }

    fn answers(&self, signal: &Signal, room: &Address) -> bool {
        signal.from() == self.on.as_str() && signal.room() == room
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitTurn {

    Idle,

    Waiting,

    Replied(crate::Letter),

    TimedOut { text: String },
}

impl SignalDesk {

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
