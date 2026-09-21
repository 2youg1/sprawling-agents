// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Signals between residents, and the side that receives them.
//!
//! Delivery is at-least-once, so the receiving side deduplicates by the
//! signal's own id, and it does so before anything that cannot be
//! replayed: a second delivery must not bill twice or send a second
//! letter. That deduplication is the queue's `seen` set rather than a
//! table kept here, which holds only while one id always lands in one
//! lane — so a signal's lane is derived from the signal, never chosen at
//! the call site.
//!
//! Taking is the receiver's move. A sender cannot push into another
//! agent's context window; the receiver pulls at most its own bandwidth,
//! and what stands in the prefix is nothing at all — only `status`
//! reports that signals are waiting.

use kernel::{Address, Admission, AxCode, AxError, IdemKey, Payload, RunId, Seq, TimeMs, Version};
use memory::{EventQueue, QueueLane};
use serde::{Deserialize, Serialize};

mod signal_id;
mod signal_kind;

pub use signal_id::SignalId;
pub use signal_kind::SignalKind;

/// Which line a signal waits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    Urgent,
    Ordinary,
}

/// One communication between residents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    id: SignalId,
    kind: SignalKind,
    from: String,
    room: Address,
    room_version: Version,
    payload: Payload,
    at: TimeMs,
}

impl Signal {
    /// Sole constructor.
    ///
    /// # Errors
    /// Refuses a signal with no sender: `from` decides how the receiver
    /// renders it, and an unattributed signal renders as nobody.
    pub fn new(
        id: SignalId,
        kind: SignalKind,
        from: String,
        room: Address,
        room_version: Version,
        payload: Payload,
        at: TimeMs,
    ) -> Result<Signal, AxError> {
        if from.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "build a signal",
                id.as_str().to_owned(),
            )
            .with_recovery("name the sender; a signal is read as coming from someone"));
        }
        Ok(Signal {
            id,
            kind,
            from,
            room,
            room_version,
            payload,
            at,
        })
    }

    #[must_use]
    pub fn id(&self) -> &SignalId {
        &self.id
    }

    #[must_use]
    pub fn kind(&self) -> SignalKind {
        self.kind
    }

    #[must_use]
    pub fn from(&self) -> &str {
        &self.from
    }

    #[must_use]
    pub fn room(&self) -> &Address {
        &self.room
    }

    /// The room version the sender saw when speaking. A held draft is
    /// judged against it.
    #[must_use]
    pub fn room_version(&self) -> Version {
        self.room_version
    }

    #[must_use]
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    #[must_use]
    pub fn at(&self) -> TimeMs {
        self.at
    }

    /// Urgent for a steer, ordinary for everything else. Derived rather
    /// than supplied: see the module note on deduplication.
    #[must_use]
    pub fn lane(&self) -> Lane {
        match self.kind {
            SignalKind::Steer => Lane::Urgent,
            SignalKind::Mention | SignalKind::Thread | SignalKind::Broadcast => Lane::Ordinary,
        }
    }

    /// The `signal_enqueued` record.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn enqueued_payload(&self) -> Result<Payload, AxError> {
        Payload::of(&SignalLine {
            id: self.id.clone(),
            kind: self.kind,
            from: self.from.clone(),
            room: self.room.clone(),
            room_version: self.room_version,
            payload: self.payload.clone(),
            at: self.at,
            lane: Some(self.lane()),
        })
    }

    /// The `signal_consumed` record: the id and who took it, because the
    /// content is already in the enqueue record and history does not
    /// need it twice.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn consumed_payload(&self, by: &str) -> Result<Payload, AxError> {
        Payload::of(&SignalConsumed {
            id: self.id.clone(),
            by: by.to_owned(),
        })
    }

    /// Reads back what [`enqueued_payload`](Self::enqueued_payload)
    /// wrote. The inverse is public because rebuilding the queues from
    /// the ledger is the only way the city knows what is waiting after a
    /// restart, and a second parser of this shape would be a second
    /// answer to that question.
    ///
    /// # Errors
    /// Refuses a payload missing a field or carrying a kind this version
    /// does not know.
    pub fn from_payload(payload: &Payload) -> Result<Signal, AxError> {
        let line: SignalLine = payload.read()?;
        Signal::new(
            line.id,
            line.kind,
            line.from,
            line.room,
            line.room_version,
            line.payload,
            line.at,
        )
    }
}

/// `signal_enqueued`: the signal itself, and the line it waits in.
///
/// The one authority for this line's keys. They used to be written key
/// by key and read key by key in this same file, which is two spellings
/// of seven names and a place for them to drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SignalLine {
    id: SignalId,
    kind: SignalKind,
    from: String,
    room: Address,
    room_version: Version,
    payload: Payload,
    at: TimeMs,
    /// Written for a reader outside this crate, never read back here:
    /// the lane is derived from `kind` by [`Signal::lane`], and reading
    /// a stored copy would let a line say which lane it took while the
    /// derivation says another. Absent on a line written before the key
    /// existed, which changes nothing, for the same reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lane: Option<Lane>,
}

/// `signal_consumed`: which signal was taken, and by whom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalConsumed {
    pub id: SignalId,
    pub by: String,
}

impl SignalConsumed {
    /// Reads back what [`Signal::consumed_payload`] wrote.
    ///
    /// # Errors
    /// Refuses a payload this build cannot read as a consumption. A
    /// fold that skipped such a line instead would count the signal as
    /// still waiting and hand it to a resident twice.
    pub fn from_payload(payload: &Payload) -> Result<SignalConsumed, AxError> {
        payload.read()
    }
}

/// The receiving side: two lines and a bandwidth.
pub struct Inbox {
    urgent: EventQueue,
    ordinary: EventQueue,
    bandwidth: u32,
}

// Hand-written: what a reader of a failure needs is how much is waiting
// in each line, which is exactly what the queues will not print.
impl std::fmt::Debug for Inbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inbox")
            .field("urgent", &self.urgent.len())
            .field("ordinary", &self.ordinary.len())
            .field("bandwidth", &self.bandwidth)
            .finish()
    }
}

impl Inbox {
    /// `bandwidth` is how many signals one pull may take. Zero would
    /// make a receiver that never reads, so it is raised to one.
    #[must_use]
    pub fn new(capacity: u64, bandwidth: u32) -> Inbox {
        Inbox {
            urgent: EventQueue::new(QueueLane::Signal, capacity),
            ordinary: EventQueue::new(QueueLane::Signal, capacity),
            bandwidth: bandwidth.max(1),
        }
    }

    /// Delivers one signal. A repeat of an id already delivered is
    /// admitted and dropped: the sender did its job, and a refusal would
    /// only buy a retry that changes nothing.
    ///
    /// # Errors
    /// Propagates the queue's own refusal to hold the payload.
    pub fn deliver(&mut self, signal: &Signal) -> Result<Admission, AxError> {
        let key = IdemKey::derive(&RunId::CITY, Seq::FIRST, signal.id().as_str().as_bytes());
        let payload = signal.enqueued_payload()?;
        let queue = match signal.lane() {
            Lane::Urgent => &mut self.urgent,
            Lane::Ordinary => &mut self.ordinary,
        };
        queue
            .enqueue(key, payload, signal.at())
            .map_err(memory::MemoryError::into_ax)
    }

    /// Takes up to the receiver's bandwidth, urgent first.
    ///
    /// # Errors
    /// Propagates a queued payload that does not read back as a signal.
    pub fn pull(&mut self) -> Result<Vec<Signal>, AxError> {
        let mut out = Vec::new();
        while u32::try_from(out.len()).unwrap_or(u32::MAX) < self.bandwidth {
            let item = match self.urgent.consume() {
                Some(item) => item,
                None => match self.ordinary.consume() {
                    Some(item) => item,
                    None => break,
                },
            };
            out.push(Signal::from_payload(&item.payload)?);
        }
        Ok(out)
    }

    /// Takes one signal from the urgent line, or nothing when it is
    /// empty. The ordinary line is not touched.
    ///
    /// The urgent line and the steer kind are the same set — [`Signal::lane`]
    /// sends `Steer` there and nothing else — so this is how a run
    /// collects what is allowed to interrupt it without reading the mail
    /// it has not asked for yet.
    ///
    /// A queued payload that does not read back as a signal is dropped
    /// rather than returned as a failure, and the contract says so:
    /// this is called from a run's safe point, where the only
    /// alternative to moving on is stopping a run over somebody else's
    /// corrupt entry. The same payload still fails loudly through
    /// [`Inbox::pull`], which is the door the model itself uses.
    pub fn take_steer(&mut self) -> Option<Signal> {
        let item = self.urgent.consume()?;
        Signal::from_payload(&item.payload).ok()
    }

    /// What `status` reports as `signals_pending`.
    #[must_use]
    pub fn pending(&self) -> u32 {
        let total = self.urgent.len().saturating_add(self.ordinary.len());
        u32::try_from(total).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
