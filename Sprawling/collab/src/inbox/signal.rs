// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One signal between residents: what the ledger records of it, and what
//! the city stamps on it when it delivers it.

use kernel::event::record::{Lane, SignalConsumed, SignalEnqueued, SignalId, SignalKind};
use kernel::{Address, AxCode, AxError, Payload, RunId, TimeMs, Version};

/// Where the run that sent a signal stood when the city delivered it,
/// stamped at delivery and never revised: a signal is history once it
/// is sent, and the reader decides from this whether it still applies
/// (collab D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    /// The run that wrote the `signal_enqueued` line, stamped by the city
    /// when it delivers the signal (collab D16); absent on a signal
    /// rebuilt from history.
    run: Option<RunId>,
}

/// What a queue holds: the ledger's line, and what the city stamped on
/// the signal when it delivered it, which the ledger line does not carry
/// and a queued signal would otherwise lose (collab D10, D16).
#[derive(serde::Serialize, serde::Deserialize)]
struct Queued {
    #[serde(flatten)]
    line: SignalEnqueued,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sender: Option<SenderState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run: Option<RunId>,
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
            run: None,
        })
    }

    /// This signal with the run that wrote its `signal_enqueued` line,
    /// which the receiver's letter names (collab D16).
    #[must_use]
    pub fn sent_by(self, run: RunId) -> Signal {
        Signal {
            run: Some(run),
            ..self
        }
    }

    /// The run that sent this signal, where the city stamped it.
    #[must_use]
    pub fn run(&self) -> Option<RunId> {
        self.run
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
        Payload::of(&self.enqueued())
    }

    fn enqueued(&self) -> SignalEnqueued {
        SignalEnqueued {
            id: self.id.clone(),
            kind: self.kind,
            from: self.from.clone(),
            room: self.room.clone(),
            room_version: self.room_version,
            payload: self.payload.clone(),
            at: self.at,
            lane: Some(self.lane()),
        }
    }

    pub(super) fn queued_payload(&self) -> Result<Payload, AxError> {
        Payload::of(&Queued {
            line: self.enqueued(),
            sender: self.sender,
            run: self.run,
        })
    }

    pub(super) fn from_queued(payload: &Payload) -> Result<Signal, AxError> {
        let queued: Queued = payload.read()?;
        let signal = Signal::from_line(queued.line)?;
        Ok(Signal {
            sender: queued.sender,
            run: queued.run,
            ..signal
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
        Signal::from_line(payload.read()?)
    }

    fn from_line(line: SignalEnqueued) -> Result<Signal, AxError> {
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
