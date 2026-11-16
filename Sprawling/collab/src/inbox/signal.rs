// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{Lane, SignalConsumed, SignalEnqueued, SignalId, SignalKind};
use kernel::{Address, AxCode, AxError, Payload, RunId, TimeMs, Version};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SenderState {

    Running,

    Frozen,

    Cancelled,
}

impl SenderState {

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SenderState::Running => "running",
            SenderState::Frozen => "frozen",
            SenderState::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    id: SignalId,
    kind: SignalKind,
    from: String,
    room: Address,
    room_version: Version,
    payload: Payload,
    at: TimeMs,

    sender: Option<SenderState>,

    run: Option<RunId>,
}

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

    #[must_use]
    pub fn sent_by(self, run: RunId) -> Signal {
        Signal {
            run: Some(run),
            ..self
        }
    }

    #[must_use]
    pub fn run(&self) -> Option<RunId> {
        self.run
    }

    #[must_use]
    pub fn delivered(self, sender: SenderState) -> Signal {
        Signal {
            sender: Some(sender),
            ..self
        }
    }

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

    #[must_use]
    pub fn lane(&self) -> Lane {
        match self.kind {
            SignalKind::Steer => Lane::Urgent,
            SignalKind::Mention | SignalKind::Thread | SignalKind::Broadcast => Lane::Ordinary,
        }
    }

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

    pub fn consumed_payload(&self, by: &str) -> Result<Payload, AxError> {
        Payload::of(&SignalConsumed {
            id: self.id.clone(),
            by: by.to_owned(),
        })
    }

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
