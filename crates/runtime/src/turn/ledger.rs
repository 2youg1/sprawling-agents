// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The turn's only door to the ledger.
//!
//! A turn appends two kinds of payload. One it authored itself: segment
//! hashes, the model name, the operator's steer text — values this
//! module computed and can vouch for. The other it merely carried: a
//! model reply, a tool's arguments, a tool's result. The carried kind
//! routinely holds another project's `.env` line, an API key or a
//! passphrase, and the ledger is append-only and exportable, so a
//! secret that lands there outlives the session, the machine, and any
//! later decision to take it back.
//!
//! The two kinds therefore travel through two different functions of
//! [`Journal`], and the carried one is scanned on the way in.
//! [`Authored`] and [`Carried`] partition this module's event kinds,
//! neither can name the other's, and no other spelling of [`EventKind`]
//! reaches an [`EventDraft`] — which is what makes the scan structural
//! rather than remembered.

use kernel::{AxError, EventDraft, EventKind, EventRef, Ledger, Payload, RunId, TimeMs};
use serde_json::{Map, Value};

/// Events whose payload this module built from values it computed.
#[derive(Debug, Clone, Copy)]
pub(super) enum Authored {
    PromptAssembled,
    ModelCalled,
    CancelReceived,
    SteerReceived,
}

/// Events whose payload came back from a provider or from a tool, and
/// may hold a credential in plain text.
#[derive(Debug, Clone, Copy)]
pub(super) enum Carried {
    ModelReturned,
    ToolCalled,
    ToolResult,
}

impl Authored {
    fn kind(self) -> EventKind {
        match self {
            Authored::PromptAssembled => EventKind::PromptAssembled,
            Authored::ModelCalled => EventKind::ModelCalled,
            Authored::CancelReceived => EventKind::CancelReceived,
            Authored::SteerReceived => EventKind::SteerReceived,
        }
    }
}

impl Carried {
    fn kind(self) -> EventKind {
        match self {
            Carried::ModelReturned => EventKind::ModelReturned,
            Carried::ToolCalled => EventKind::ToolCalled,
            Carried::ToolResult => EventKind::ToolResult,
        }
    }
}

/// One turn's line of history: the stamp every event of the turn
/// shares, the refs it has collected, and how many secret-shaped spans
/// it kept out of the ledger.
///
/// The phase data lives beside this in `Turn`, not inside it, so a
/// phase change can move the phase out while the journal stays put and
/// keeps appending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Journal {
    run: RunId,
    who: String,
    t: TimeMs,
    refs: Vec<EventRef>,
    redacted: u32,
}

impl Journal {
    pub(super) fn open(run: RunId, who: String, t: TimeMs) -> Journal {
        Journal {
            run,
            who,
            t,
            refs: Vec::new(),
            redacted: 0,
        }
    }

    /// How many secret-shaped spans this turn replaced so far.
    pub(super) fn redacted(&self) -> u32 {
        self.redacted
    }

    /// Hands over the refs collected so far, leaving the journal empty:
    /// the one way a turn's refs reach a report or a cancellation.
    pub(super) fn take_refs(&mut self) -> Vec<EventRef> {
        let mut refs = std::mem::take(&mut self.refs);
        refs.shrink_to_fit();
        refs
    }

    /// Appends an event the turn authored, verbatim, and keeps its ref.
    ///
    /// # Errors
    /// Propagates the ledger's refusal to record the event.
    pub(super) fn append_authored(
        &mut self,
        ledger: &mut dyn Ledger,
        event: Authored,
        data: Payload,
    ) -> Result<EventRef, AxError> {
        self.append(ledger, event.kind(), data)
    }

    /// Appends an event the turn carried, with every secret-shaped span
    /// replaced by a `secret:redacted/<b3-16>` marker first, and keeps
    /// its ref.
    ///
    /// The window already holds the blocks the next request is built
    /// from, so replacing here cannot break a thinking block's
    /// signature or make the conversation stop making sense: history
    /// and context are two sinks, and only this one is permanent.
    ///
    /// # Errors
    /// Reports `E_INVALID_ARGS` when the scanned map is not a valid
    /// payload, and propagates the ledger's refusal to record the
    /// event.
    pub(super) fn append_redacted(
        &mut self,
        ledger: &mut dyn Ledger,
        event: Carried,
        data: Map<String, Value>,
    ) -> Result<EventRef, AxError> {
        let (scanned, hits) = crate::redact::redact(&data);
        self.redacted = self.redacted.saturating_add(hits);
        self.append(ledger, event.kind(), Payload::new(scanned)?)
    }

    /// The turn module's single `Ledger::append` call, and the single
    /// place an [`EventDraft`] of this turn is built: the stamp, the
    /// author and the ref bookkeeping cannot drift between phases.
    fn append(
        &mut self,
        ledger: &mut dyn Ledger,
        kind: EventKind,
        data: Payload,
    ) -> Result<EventRef, AxError> {
        let echo = ledger.append(EventDraft {
            run: self.run,
            t: self.t,
            who: self.who.clone(),
            addr: None,
            kind,
            data,
            ig: false,
        })?;
        self.refs.push(echo);
        Ok(echo)
    }
}
