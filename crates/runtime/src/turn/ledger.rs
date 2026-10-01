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
//!
//! The same variants decide when a line happened. The four lines a turn
//! waited for - a model attempt sent, its reply whole, a tool call
//! started, its answer - carry their own reading and cannot be built
//! without one; every other line carries the turn's stamp. The clock
//! those readings come from is read in this module and nowhere else in
//! the turn (`crates/kernel/Spec.lean` §8-4, "what the envelope `t` records").

use kernel::{AxError, EventDraft, EventKind, EventRef, Ledger, Payload, RunId, TimeMs};

/// Events whose payload this module built from values it computed.
#[derive(Debug, Clone, Copy)]
pub(super) enum Authored {
    PromptAssembled,
    /// A model attempt, sent at `at`.
    ModelCalled {
        at: TimeMs,
    },
    CancelReceived,
    SteerReceived,
}

/// Events whose payload came back from a provider or from a tool, and
/// may hold a credential in plain text. Each is a moment the turn waited
/// for, so each carries its own reading.
#[derive(Debug, Clone, Copy)]
pub(super) enum Carried {
    /// A model reply, whole at `at`.
    ModelReturned { at: TimeMs },
    /// A tool call, started at `at`.
    ToolCalled { at: TimeMs },
    /// A tool call's answer, recorded at `at`.
    ToolResult { at: TimeMs },
}

impl Authored {
    fn kind(self) -> EventKind {
        match self {
            Authored::PromptAssembled => EventKind::PromptAssembled,
            Authored::ModelCalled { .. } => EventKind::ModelCalled,
            Authored::CancelReceived => EventKind::CancelReceived,
            Authored::SteerReceived => EventKind::SteerReceived,
        }
    }

    /// When the line happened: its own reading for the attempt the turn
    /// waited on, the turn's stamp for every other line.
    fn at(self, turn: TimeMs) -> TimeMs {
        match self {
            Authored::ModelCalled { at } => at,
            Authored::PromptAssembled | Authored::CancelReceived | Authored::SteerReceived => turn,
        }
    }
}

impl Carried {
    fn kind(self) -> EventKind {
        match self {
            Carried::ModelReturned { .. } => EventKind::ModelReturned,
            Carried::ToolCalled { .. } => EventKind::ToolCalled,
            Carried::ToolResult { .. } => EventKind::ToolResult,
        }
    }

    /// When the line happened: every carried line is a moment the turn
    /// waited for.
    fn at(self) -> TimeMs {
        match self {
            Carried::ModelReturned { at }
            | Carried::ToolCalled { at }
            | Carried::ToolResult { at } => at,
        }
    }
}

/// One turn's line of history: the turn's stamp, the clock the lines it
/// waited for are read from, the refs it has collected, and how many
/// secret-shaped spans it kept out of the ledger.
///
/// The phase data lives beside this in `Turn`, not inside it, so a
/// phase change can move the phase out while the journal stays put and
/// keeps appending.
pub(super) struct Journal<'h> {
    run: RunId,
    who: String,
    t: TimeMs,
    now: &'h mut dyn FnMut() -> Result<TimeMs, AxError>,
    refs: Vec<EventRef>,
    redacted: u32,
}

impl std::fmt::Debug for Journal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Journal")
            .field("run", &self.run)
            .field("who", &self.who)
            .field("t", &self.t)
            .field("refs", &self.refs)
            .field("redacted", &self.redacted)
            .finish_non_exhaustive()
    }
}

impl<'h> Journal<'h> {
    pub(super) fn open(
        run: RunId,
        who: String,
        t: TimeMs,
        now: &'h mut dyn FnMut() -> Result<TimeMs, AxError>,
    ) -> Journal<'h> {
        Journal {
            run,
            who,
            t,
            now,
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
        let at = event.at(self.t);
        self.append(ledger, event.kind(), at, data)
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
        data: Payload,
    ) -> Result<EventRef, AxError> {
        let (scanned, hits) = crate::redact::redact(data.as_map());
        self.redacted = self.redacted.saturating_add(hits);
        self.append(ledger, event.kind(), event.at(), Payload::new(scanned)?)
    }

    /// The turn's stamp: the time every line of this turn carries except
    /// the four it waited for, and the time a wave's tools are admitted at.
    pub(super) fn stamp(&self) -> TimeMs {
        self.t
    }

    /// One reading of the clock the driver handed to the turn, for a
    /// line the turn waited for.
    ///
    /// # Errors
    /// Propagates the clock's failure, such as a clock past `u64`.
    pub(super) fn read_clock(&mut self) -> Result<TimeMs, AxError> {
        (self.now)()
    }

    /// The turn module's single `Ledger::append` call, and the single
    /// place an [`EventDraft`] of this turn is built: the time, the
    /// author and the ref bookkeeping cannot drift between phases.
    fn append(
        &mut self,
        ledger: &mut dyn Ledger,
        kind: EventKind,
        at: TimeMs,
        data: Payload,
    ) -> Result<EventRef, AxError> {
        let echo = ledger.append(EventDraft {
            run: self.run,
            t: at,
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
