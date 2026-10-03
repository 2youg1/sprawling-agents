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

use kernel::event::record::{Lane, SignalConsumed, SignalEnqueued, SignalId, SignalKind};
use kernel::{Address, Admission, AxCode, AxError, IdemKey, Payload, RunId, Seq, TimeMs, Version};
use storage::{EventQueue, QueueLane};

/// Where the run that sent a signal stood when the city delivered it,
/// stamped at delivery and never revised: a signal is history once it
/// is sent, and the reader decides from this whether it still applies
/// (collab D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderState {
    /// The sender was still driving: an ordinary send.
    Running,
    /// The sender had frozen with its work done or at its limit.
    Frozen,
    /// The sender had been cancelled.
    Cancelled,
}

impl SenderState {
    /// The word the reader is shown after the sender's `@address`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SenderState::Running => "running",
            SenderState::Frozen => "frozen",
            SenderState::Cancelled => "cancelled",
        }
    }
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
    /// Stamped by the city when it delivers the signal; absent on a
    /// signal rebuilt from the history, whose sender's state at delivery
    /// the history does not carry.
    sender: Option<SenderState>,
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
            sender: None,
        })
    }

    /// This signal as the city delivers it, with where its sender stood
    /// at that moment (collab D10).
    #[must_use]
    pub fn delivered(self, sender: SenderState) -> Signal {
        Signal {
            sender: Some(sender),
            ..self
        }
    }

    /// Where the sender stood when the city delivered this signal.
    #[must_use]
    pub fn sender(&self) -> Option<SenderState> {
        self.sender
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
        Payload::of(&SignalEnqueued {
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
        let line: SignalEnqueued = payload.read()?;
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

/// The receiving side: two lines, what a leaving run gave back, and a
/// bandwidth.
pub struct Inbox {
    urgent: EventQueue,
    ordinary: EventQueue,
    /// Signals a run took and left with before any recorded answer read
    /// them (collab D8). They are older than anything in the two lines,
    /// so they are taken first; they bypass the lines because the
    /// lines' `seen` set would drop a second delivery of the same id.
    returned: std::collections::VecDeque<Signal>,
    bandwidth: u32,
}

// Hand-written: what a reader of a failure needs is how much is waiting
// in each line, which is exactly what the queues will not print.
impl std::fmt::Debug for Inbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inbox")
            .field("urgent", &self.urgent.len())
            .field("ordinary", &self.ordinary.len())
            .field("returned", &self.returned.len())
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
            returned: std::collections::VecDeque::new(),
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
            .map_err(storage::StorageError::into_ax)
    }

    /// Puts back a signal a run took and left with unread, ahead of
    /// everything still queued, because it was sent before all of it
    /// (`spec/Delivery.lean` `leave_requeues`).
    pub fn give_back(&mut self, signals: Vec<Signal>) {
        for signal in signals.into_iter().rev() {
            self.returned.push_front(signal);
        }
    }

    /// Takes up to the receiver's bandwidth: what was given back first,
    /// then urgent, then ordinary.
    ///
    /// # Errors
    /// Propagates a queued payload that does not read back as a signal.
    pub fn pull(&mut self) -> Result<Vec<Signal>, AxError> {
        let mut out = Vec::new();
        while u32::try_from(out.len()).unwrap_or(u32::MAX) < self.bandwidth {
            if let Some(signal) = self.returned.pop_front() {
                out.push(signal);
                continue;
            }
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
        let given_back = self
            .returned
            .iter()
            .position(|signal| matches!(signal.lane(), Lane::Urgent));
        if let Some(signal) = given_back.and_then(|at| self.returned.remove(at)) {
            return Some(signal);
        }
        let item = self.urgent.consume()?;
        Signal::from_payload(&item.payload).ok()
    }

    /// What `status` reports as `signals_pending`.
    #[must_use]
    pub fn pending(&self) -> u32 {
        let total = self.urgent.len().saturating_add(self.ordinary.len());
        u32::try_from(total)
            .unwrap_or(u32::MAX)
            .saturating_add(u32::try_from(self.returned.len()).unwrap_or(u32::MAX))
    }
}

/// Where the city drops a signal for a room whose queue is lent to a
/// running run, so that run finds it at its next safe point (collab D7).
///
/// Shared between the accounting thread, which drops signals in, and the
/// run's signal desk, which empties it into the lent queue. It is a
/// lock of its own rather than the desk's, because a desk is locked for
/// the length of a tool call and a `send` from that call blocks until
/// the accounting thread has delivered it, which may be into this room.
#[derive(Debug, Clone, Default)]
pub struct Mailslot(std::sync::Arc<std::sync::Mutex<Vec<Signal>>>);

impl Mailslot {
    /// Drops one signal in, or sheds it once `capacity` are waiting.
    ///
    /// # Errors
    /// Refuses when the slot was left locked by a thread that died.
    pub fn drop_in(&self, signal: Signal, capacity: u64) -> Result<Admission, AxError> {
        let mut waiting = self.0.lock().map_err(|_| poisoned())?;
        if u64::try_from(waiting.len()).unwrap_or(u64::MAX) >= capacity {
            return Ok(Admission::Shed {
                reason: kernel::ShedReason::CapacityExhausted,
            });
        }
        waiting.push(signal);
        Ok(Admission::Admit)
    }

    /// Takes everything waiting, in the order it arrived.
    ///
    /// # Errors
    /// Refuses when the slot was left locked by a thread that died.
    pub fn take(&self) -> Result<Vec<Signal>, AxError> {
        Ok(std::mem::take(&mut *self.0.lock().map_err(|_| poisoned())?))
    }
}

fn poisoned() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "reach a room's mailslot",
        "the slot was left locked by a thread that died",
    )
    .with_recovery("restart this city")
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
