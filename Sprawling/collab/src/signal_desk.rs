// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, Payload, RunId, TimeMs, Version};
use serde_json::{Map, Value};

use crate::inbox::{Inbox, Mailslot, Signal};
use kernel::event::record::{SignalId, SignalKind, SignalWaitEnded, SignalWaitStarted};

use crate::reply_wait::ReplyWait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalEffect {
    Enqueued(Signal),

    Consumed { signal: Signal, by: String },

    WaitStarted(SignalWaitStarted),

    WaitEnded(SignalWaitEnded),
}

pub struct Post(pub(crate) Box<Write>);

type Write = dyn FnMut(&SignalEffect) -> Result<(), AxError> + Send;

impl Post {
    #[must_use]
    pub fn new(write: impl FnMut(&SignalEffect) -> Result<(), AxError> + Send + 'static) -> Post {
        Post(Box::new(write))
    }
}

impl std::fmt::Debug for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Post")
    }
}

#[derive(Debug)]
pub struct RoomMail {
    pub inbox: Inbox,
    pub slot: Mailslot,
    pub post: Post,
}

pub struct SignalDesk {
    run: RunId,
    pub(crate) room: Address,
    pub(crate) who: String,
    reach: Address,
    at: TimeMs,
    pub(crate) inbox: Inbox,
    pub(crate) slot: Mailslot,
    pub(crate) post: Post,
    pub(crate) held: Vec<Signal>,
    pub(crate) waiting: Option<ReplyWait>,
    unread: Vec<Signal>,
    minted: u32,
}

impl std::fmt::Debug for SignalDesk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalDesk")
            .field("room", &self.room)
            .field("pending", &self.inbox.pending())
            .field("held", &self.held.len())
            .finish()
    }
}

impl SignalDesk {
    #[must_use]
    pub fn new(
        run: RunId,
        room: Address,
        who: String,
        reach: Address,
        at: TimeMs,
        mail: RoomMail,
    ) -> SignalDesk {
        let RoomMail { inbox, slot, post } = mail;
        SignalDesk {
            run,
            room,
            who,
            reach,
            at,
            inbox,
            slot,
            post,
            held: Vec::new(),
            waiting: None,
            unread: Vec::new(),
            minted: 0,
        }
    }

    #[must_use]
    pub fn pending(&self) -> u32 {
        self.inbox.pending()
    }

    pub fn take_steer(&mut self) -> Result<Option<crate::Letter>, AxError> {
        self.collect()?;
        let Some(signal) = self.inbox.take_steer() else {
            return Ok(None);
        };
        match crate::Letter::from_signal(&signal) {
            Ok(steer) => {
                self.held.push(signal);
                Ok(Some(steer))
            }
            Err(unreadable) => {
                (self.post.0)(&SignalEffect::Consumed {
                    signal,
                    by: self.who.clone(),
                })?;
                Err(unreadable)
            }
        }
    }

    pub fn answered(&mut self) -> Result<(), AxError> {
        let mut held = std::mem::take(&mut self.held).into_iter();
        while let Some(signal) = held.next() {
            let line = SignalEffect::Consumed {
                signal,
                by: self.who.clone(),
            };
            if let Err(refused) = (self.post.0)(&line) {
                if let SignalEffect::Consumed { signal, .. } = line {
                    self.held = std::iter::once(signal).chain(held).collect();
                }
                return Err(refused);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn take_inbox(&mut self) -> Inbox {
        let mut unread = std::mem::take(&mut self.held);
        unread.extend(self.waiting.as_mut().and_then(ReplyWait::unkeep));
        if let Ok(arrived) = self.slot.take() {
            unread.extend(arrived);
        }
        self.inbox.give_back(unread.clone());
        self.unread = unread;
        std::mem::replace(&mut self.inbox, Inbox::new(0, 1))
    }

    pub fn take_unread(&mut self) -> Vec<Signal> {
        std::mem::take(&mut self.unread)
    }

    fn collect(&mut self) -> Result<(), AxError> {
        for signal in self.slot.take()? {
            let queued = match self.waiting.as_mut() {
                Some(wait) => wait.keep(signal, &self.room),
                None => Some(signal),
            };
            if let Some(signal) = queued {
                self.admit(&signal)?;
            }
        }
        Ok(())
    }

    pub(crate) fn admit(&mut self, signal: &Signal) -> Result<(), AxError> {
        match self.inbox.deliver(signal)? {
            kernel::Admission::Shed { .. } => Err(AxError::failure(
                AxCode::BackpressureShed,
                "deliver a signal to a running room",
                format!("room {} shed signal {}", self.room, signal.id().as_str()),
            )
            .with_recovery("the queue is full; pull what is waiting, then ask again")),
            kernel::Admission::Admit => Ok(()),
        }
    }

    fn mint(&mut self) -> Result<SignalId, AxError> {
        self.minted = self.minted.checked_add(1).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "mint a signal id",
                "this run has sent as many signals as one run can",
            )
            .with_recovery("freeze the run and dispatch again")
        })?;
        SignalId::parse(&format!("{}-s{}", self.run, self.minted))
    }

    pub(crate) fn send(&mut self, args: &Map<String, Value>) -> Result<Payload, AxError> {
        let to = Address::parse(text(args, "to", "send a signal")?)?;
        if !to.is_within(&self.reach) {
            return Err(AxError::failure(
                AxCode::CrossBuildingDenied,
                "send a signal",
                to.as_str().to_owned(),
            )
            .with_recovery(format!(
                "signal an address inside {}, or ask the User to carry it across",
                self.reach.as_str()
            )));
        }
        let kind = match args.get("kind").and_then(Value::as_str) {
            Some(raw) => SignalKind::parse(raw)?,
            None => SignalKind::Mention,
        };
        let body = text(args, "text", "send a signal")?.to_owned();
        let mut payload = Map::new();
        payload.insert("text".to_owned(), Value::String(body));
        let signal = Signal::new(
            self.mint()?,
            kind,
            self.who.clone(),
            to.clone(),
            Version::FIRST,
            Payload::new(payload)?,
            self.at,
        )?;
        let mut result = Map::new();
        result.insert(
            "id".to_owned(),
            Value::String(signal.id().as_str().to_owned()),
        );
        result.insert("to".to_owned(), Value::String(to.as_str().to_owned()));
        result.insert(
            "kind".to_owned(),
            Value::String(signal.kind().as_str().to_owned()),
        );
        let wait = crate::reply_wait::wanted(args, self.waiting.is_some())?
            .then(|| ReplyWait::asked(to, signal.id().clone()));
        (self.post.0)(&SignalEffect::Enqueued(signal))?;
        result.insert("delivered".to_owned(), Value::Bool(true));
        if let Some(wait) = wait {
            result.insert("waiting".to_owned(), Value::String(wait.promise()));
            self.waiting = Some(wait);
        }
        Payload::new(result)
    }

    pub(crate) fn pull(&mut self) -> Result<Payload, AxError> {
        self.collect()?;
        let taken = self.inbox.pull()?;
        let mut rows = Vec::with_capacity(taken.len());
        for signal in taken {
            let mut row = Map::new();
            row.insert("from".to_owned(), Value::String(signal.from().to_owned()));
            row.insert(
                "kind".to_owned(),
                Value::String(signal.kind().as_str().to_owned()),
            );
            row.insert(
                "text".to_owned(),
                signal
                    .payload()
                    .as_map()
                    .get("text")
                    .cloned()
                    .unwrap_or(Value::String(String::new())),
            );
            if let Some(state) = signal.sender() {
                row.insert(
                    "sender".to_owned(),
                    Value::String(crate::steer::sender_note(state)),
                );
            }
            rows.push(Value::Object(row));
            self.held.push(signal);
        }
        let mut result = Map::new();
        result.insert("signals".to_owned(), Value::Array(rows));

        result.insert(
            "remaining".to_owned(),
            Value::Number(self.inbox.pending().into()),
        );
        Payload::new(result)
    }
}

pub(crate) fn text<'a>(
    args: &'a Map<String, Value>,
    key: &str,
    action: &str,
) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            action.to_owned(),
            format!("missing string argument `{key}`"),
        )
        .with_recovery(format!("pass `{key}` as a string"))
    })
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
mod tests;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod delivery_tests;
