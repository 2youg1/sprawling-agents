// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The run's side of the city's signal traffic: the room's queue on
//! loan, the post every signal line goes through, and the signals the
//! run holds.
//!
//! Three orderings carry the design. A send takes effect at the call
//! (collab D7): the desk hands the signal to its [`Post`], which puts
//! the `signal_enqueued` line on the ledger and only then delivers it,
//! because a projection may only change as a consequence of a record
//! that already exists. A signal taken out of the queue is *held* until
//! a recorded model answer has read it (collab D8): only then is its
//! `signal_consumed` line written, and a run that leaves first gives it
//! back. And the room's inbox is *lent* to the desk for the length of
//! the run rather than copied into it: two queues would be two
//! authorities on what order signals arrive in, and the one that drifts
//! is always the one nobody reads.
//!
//! Reach is handed in, not worked out here. Which building an address
//! belongs to is `city`'s answer, and this crate cannot name that crate;
//! what it can do is refuse anything outside the boundary it was given.

use kernel::{Address, AxCode, AxError, Payload, RunId, TimeMs, Version};
use serde_json::{Map, Value};

use crate::inbox::{Inbox, Mailslot, Signal};
use kernel::event::record::{SignalId, SignalKind};

/// One line of the city's signal traffic, handed to the [`Post`] at the
/// moment it happens.
///
/// Exhaustive on purpose, unlike most enums that cross a crate boundary
/// here: every variant is something the post must write down, so a new
/// one has to be a compile error at the place that writes, not a runtime
/// arm nobody reaches until a signal quietly goes unrecorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalEffect {
    /// Sent: the line goes on the ledger, then the signal into its room.
    Enqueued(Signal),
    /// Read by a recorded model answer, or taken and unreadable.
    Consumed { signal: Signal, by: String },
}

/// The authority that records a signal line at the call and acts on it:
/// in the city, the relay to the accounting thread, which appends the
/// line and delivers what was sent. Its refusal reaches the model.
pub struct Post(Box<Write>);

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

/// What a run is lent of its room's mail: the queue, the slot the city
/// drops new signals into while the run holds the queue, and the post
/// its own lines go through. The three travel together because a desk
/// with any one missing either loses mail or records none.
#[derive(Debug)]
pub struct RoomMail {
    pub inbox: Inbox,
    pub slot: Mailslot,
    pub post: Post,
}

/// The run's side of the city's signal traffic: its own inbox, on loan,
/// and the signals it has taken that no recorded answer has read yet.
pub struct SignalDesk {
    run: RunId,
    pub(crate) room: Address,
    who: String,
    reach: Address,
    at: TimeMs,
    inbox: Inbox,
    slot: Mailslot,
    post: Post,
    held: Vec<Signal>,
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
    /// `reach` bounds where this run may send: an address outside it is
    /// refused. `at` is the run's stamp — the tool face has no clock, and
    /// time only ever arrives as a parameter.
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
            unread: Vec::new(),
            minted: 0,
        }
    }

    /// What `status` reports as `signals_pending` for this room.
    #[must_use]
    pub fn pending(&self) -> u32 {
        self.inbox.pending()
    }

    /// Takes one steer waiting for this run, ready to land at its next
    /// safe point, or `None` when nothing is waiting for it.
    ///
    /// The landing is the same one the person's steer uses, and the
    /// attribution is what keeps them apart: [`crate::Steer::from_signal`]
    /// can only write `@<the sender's address>`, and only
    /// `Steer::from_person` can write `user`. A resident reading its own
    /// window can therefore tell who spoke, and the prefix it reads is
    /// the address it answers to — which is what makes a reply
    /// possible at all.
    ///
    /// A steer the run can read is *held*, not consumed: the model reads
    /// it in its next request, and [`SignalDesk::answered`] records the
    /// consumption once that answer is on the ledger (collab D8). A
    /// steer the run cannot read is recorded as consumed at once,
    /// because no model will read it and delivering it again would only
    /// be refused again at the next safe point.
    ///
    /// # Errors
    /// Propagates [`crate::Steer::from_signal`]'s refusal when a consumed
    /// signal is not a steer this run can read. `Ok(None)` and `Err` are
    /// two facts — an empty queue and a message that arrived unreadable —
    /// and folding the second into the first is what this refuses to do.
    /// How a safe point treats the refusal is the caller's decision, and
    /// a slot or a post that fails is refused the same way.
    pub fn take_steer(&mut self) -> Result<Option<crate::Steer>, AxError> {
        self.collect()?;
        let Some(signal) = self.inbox.take_steer() else {
            return Ok(None);
        };
        match crate::Steer::from_signal(&signal) {
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

    /// Records that a model answer has landed since every signal now
    /// held was taken, so each of them has been read: one
    /// `signal_consumed` line per signal, written now (collab D8).
    ///
    /// # Errors
    /// Propagates the post's refusal; the signals not yet recorded stay
    /// held, so a later answer or the run's leaving still accounts for
    /// them.
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

    /// Gives the room its inbox back, with every signal this run held
    /// unread and every one still in the slot put back at the front.
    /// The caller must do this on both the failing and the succeeding
    /// path — an inbox left in a dropped desk is a queue the city forgot
    /// it had. A slot that cannot be read is left for the room table,
    /// which reads it again when the queue comes home.
    #[must_use]
    pub fn take_inbox(&mut self) -> Inbox {
        let mut unread = std::mem::take(&mut self.held);
        if let Ok(arrived) = self.slot.take() {
            unread.extend(arrived);
        }
        self.inbox.give_back(unread.clone());
        self.unread = unread;
        std::mem::replace(&mut self.inbox, Inbox::new(0, 1))
    }

    /// The signals [`SignalDesk::take_inbox`] put back unread: the room
    /// they are in is knocked once, so somebody reads them
    /// (`spec/Delivery.lean` `leave_requeues`).
    pub fn take_unread(&mut self) -> Vec<Signal> {
        std::mem::take(&mut self.unread)
    }

    /// Moves what the city dropped into the slot since the last safe
    /// point into the lent queue.
    fn collect(&mut self) -> Result<(), AxError> {
        for signal in self.slot.take()? {
            if let kernel::Admission::Shed { .. } = self.inbox.deliver(&signal)? {
                return Err(AxError::failure(
                    AxCode::BackpressureShed,
                    "deliver a signal to a running room",
                    format!("room {} shed signal {}", self.room, signal.id().as_str()),
                )
                .with_recovery("the queue is full; pull what is waiting, then ask again"));
            }
        }
        Ok(())
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
            // No room carries a version until drafts have a writer: the
            // sender saw a room nobody has revised.
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
        (self.post.0)(&SignalEffect::Enqueued(signal))?;
        result.insert("delivered".to_owned(), Value::Bool(true));
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
        // The count in `status` is the fact as it stood when the run was
        // dispatched; this one is the fact as it stands now.
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
