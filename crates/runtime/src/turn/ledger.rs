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
//! rather than remembered. The run's own lines between the turn's
//! phases are a third kind, [`RunLine`]: the run authored them, so they
//! are not scanned, and they carry the run's address.
//!
//! The same variants decide when a line happened. The four lines a turn
//! waited for - a model attempt sent, its reply whole, a tool call
//! started, its answer - carry their own reading and cannot be built
//! without one; every other line carries the turn's stamp. The clock
//! those readings come from is read in this module and nowhere else in
//! the turn (`crates/kernel/Spec.lean` §8-4, "what the envelope `t` records").

use kernel::{
    Address, AxCode, AxError, EventDraft, EventKind, EventRef, Ledger, Payload, RunId, TimeMs,
};

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

/// A line the run writes between two of the turn's phases. It goes
/// through the turn's journal so that it takes its place among the
/// turn's held lines rather than ahead of them, which is what lets those
/// lines wait for the next barrier instead of going down first
/// (`crates/runtime/spec/Turn/Durability.lean`, `tf1Turn`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum RunLine {
    /// What the assembled request looks like to a prompt cache.
    PromptShapeCompared,
    /// The commit taken before a wave that may write.
    CheckpointCommitted,
}

impl RunLine {
    fn kind(self) -> EventKind {
        match self {
            RunLine::PromptShapeCompared => EventKind::PromptShapeCompared,
            RunLine::CheckpointCommitted => EventKind::CheckpointCommitted,
        }
    }
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

/// One reading of the turn's two clocks. `us` is absent on a turn the
/// driver gave no monotonic clock, and then no duration is recorded.
#[derive(Debug, Clone, Copy)]
pub(super) struct Moment {
    pub(super) at: TimeMs,
    pub(super) us: Option<u64>,
}

impl Moment {
    /// Whole microseconds from `earlier` to this reading.
    pub(super) fn since(self, earlier: Option<u64>) -> Option<u64> {
        Some(self.us?.saturating_sub(earlier?))
    }
}

/// The run's line of history: the lines appended but not yet durable,
/// and the refs of the lines already durable, in the order they were
/// appended. It lives as long as the run (`Run<Active>`) and every turn
/// borrows it, so a turn's last lines wait for the barrier of the next
/// turn instead of paying one of their own (runtime D36).
///
/// [`HeldLines::barrier`] hands every held line to the ledger in one
/// `Ledger::append_all` and only then keeps their refs, so a ref it hands
/// out names a durable record. A turn calls it before each outside
/// effect - a model call, a write - and the run calls it once more before
/// it freezes, and nowhere else (runtime D24, D36,
/// `crates/runtime/spec/Turn/Durability.lean`).
#[derive(Debug, Clone, PartialEq)]
pub struct HeldLines {
    run: RunId,
    who: String,
    held: Vec<EventDraft>,
    refs: Vec<EventRef>,
}

/// A line this run appended, by its place among the run's lines: what a
/// phase or a report keeps in place of a ref it cannot have until the
/// next barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry(usize);

impl HeldLines {
    /// The lines of run `run`, written by `who`; nothing is held yet.
    pub fn open(run: RunId, who: String) -> HeldLines {
        HeldLines {
            run,
            who,
            held: Vec::new(),
            refs: Vec::new(),
        }
    }

    /// Every held line, in the order it was appended, through one
    /// `Ledger::append_all`. Holding nothing, it asks the ledger nothing.
    ///
    /// # Errors
    /// Propagates the ledger's refusal; the lines it refused are dropped,
    /// and the run ends on that error.
    pub fn barrier(&mut self, ledger: &mut dyn Ledger) -> Result<(), AxError> {
        if self.held.is_empty() {
            return Ok(());
        }
        let echoes = ledger.append_all(std::mem::take(&mut self.held))?;
        self.refs.extend(echoes);
        Ok(())
    }

    /// The refs of every line already made durable, in appending order.
    pub fn refs(&self) -> &[EventRef] {
        &self.refs
    }

    /// The ref of a line already made durable.
    ///
    /// # Errors
    /// Reports `E_INVALID_ARGS` when the line is still held: a ref for a
    /// line no barrier has carried would name a history that may not exist.
    pub fn durable(&self, entry: Entry) -> Result<EventRef, AxError> {
        self.refs.get(entry.0).copied().ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a run line's ref",
                "the line is not durable yet",
            )
            .with_recovery(
                "report this against runtime::turn::ledger: a ref was asked for \
                 before the barrier that carries its line",
            )
        })
    }

    /// The single place an [`EventDraft`] of this run's turns is built:
    /// the author and the place among the run's lines cannot drift
    /// between turns.
    fn hold(&mut self, kind: EventKind, at: TimeMs, addr: Option<Address>, data: Payload) -> Entry {
        let entry = Entry(self.refs.len().saturating_add(self.held.len()));
        self.held.push(EventDraft {
            run: self.run,
            t: at,
            who: self.who.clone(),
            addr,
            kind,
            data,
            ig: false,
        });
        entry
    }
}

/// One turn's pen on the run's lines: the turn's stamp, the clock the
/// lines it waited for are read from, and how many secret-shaped spans
/// it kept out of the ledger.
///
/// The phase data lives beside this in `Turn`, not inside it, so a
/// phase change can move the phase out while the journal stays put and
/// keeps appending.
pub(super) struct Journal<'h> {
    lines: &'h mut HeldLines,
    t: TimeMs,
    now: &'h mut dyn FnMut() -> Result<TimeMs, AxError>,
    stopwatch: Option<&'h mut dyn FnMut() -> u64>,
    redacted: u32,
}

impl std::fmt::Debug for Journal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Journal")
            .field("lines", &self.lines)
            .field("t", &self.t)
            .field("redacted", &self.redacted)
            .finish_non_exhaustive()
    }
}

impl<'h> Journal<'h> {
    pub(super) fn open(
        lines: &'h mut HeldLines,
        t: TimeMs,
        now: &'h mut dyn FnMut() -> Result<TimeMs, AxError>,
    ) -> Journal<'h> {
        Journal {
            lines,
            t,
            now,
            stopwatch: None,
            redacted: 0,
        }
    }

    /// Times the turn's waits on `monotonic_us` from here on: microseconds
    /// on a clock that only moves forward, from an origin of the
    /// driver's choosing (kernel D20).
    pub(super) fn time_with(&mut self, monotonic_us: &'h mut dyn FnMut() -> u64) {
        self.stopwatch = Some(monotonic_us);
    }

    /// How many secret-shaped spans this turn replaced so far.
    pub(super) fn redacted(&self) -> u32 {
        self.redacted
    }

    /// The run's lines this turn writes to.
    pub(super) fn lines(&mut self) -> &mut HeldLines {
        self.lines
    }

    /// The barrier a turn pays before an outside effect
    /// ([`HeldLines::barrier`]).
    ///
    /// # Errors
    /// Propagates the ledger's refusal.
    pub(super) fn barrier(&mut self, ledger: &mut dyn Ledger) -> Result<(), AxError> {
        self.lines.barrier(ledger)
    }

    /// Appends an event the turn authored, verbatim. The line is held
    /// until the next [`Journal::barrier`].
    pub(super) fn append_authored(&mut self, event: Authored, data: Payload) -> Entry {
        let at = event.at(self.t);
        self.append(event.kind(), at, None, data)
    }

    /// Appends a line the run wrote at `addr`, at the turn's stamp. The
    /// line is held until the next [`Journal::barrier`].
    pub(super) fn append_run_line(&mut self, line: RunLine, addr: Address, data: Payload) -> Entry {
        self.append(line.kind(), self.t, Some(addr), data)
    }

    /// Appends an event the turn carried, with every secret-shaped span
    /// replaced by a `secret:redacted/<b3-16>` marker first. The line is
    /// held until the next [`Journal::barrier`].
    ///
    /// The window already holds the blocks the next request is built
    /// from, so replacing here cannot break a thinking block's
    /// signature or make the conversation stop making sense: history
    /// and context are two sinks, and only this one is permanent.
    ///
    /// # Errors
    /// Reports `E_INVALID_ARGS` when the scanned map is not a valid
    /// payload.
    pub(super) fn append_redacted(
        &mut self,
        event: Carried,
        data: Payload,
    ) -> Result<Entry, AxError> {
        let (scanned, hits) = crate::redact::redact(data.into_map());
        self.redacted = self.redacted.saturating_add(hits);
        Ok(self.append(event.kind(), event.at(), None, Payload::new(scanned)?))
    }

    /// The turn's stamp: the time every line of this turn carries except
    /// the four it waited for, and the time a wave's tools are admitted at.
    pub(super) fn stamp(&self) -> TimeMs {
        self.t
    }

    /// One reading of both clocks: the moment a line carries, and the
    /// monotonic microseconds a duration is the difference of.
    ///
    /// # Errors
    /// Propagates the clock's failure, such as a clock past `u64`.
    pub(super) fn read_moment(&mut self) -> Result<Moment, AxError> {
        Ok(Moment {
            at: (self.now)()?,
            us: self.stopwatch.as_mut().map(|read| read()),
        })
    }

    fn append(
        &mut self,
        kind: EventKind,
        at: TimeMs,
        addr: Option<Address>,
        data: Payload,
    ) -> Entry {
        self.lines.hold(kind, at, addr, data)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod a_ref_before_its_barrier {
    use super::*;

    /// A ref asked for before the barrier names a history that may not
    /// exist; the refusal is one sentence.
    #[test]
    fn is_refused_in_one_sentence() {
        let refused = HeldLines::open(RunId::from_bytes([7; 16]), "resident@sim.1".to_owned())
            .durable(Entry(0))
            .unwrap_err();
        assert_eq!(
            refused.recovery(),
            "report this against runtime::turn::ledger: a ref was asked for before the barrier that carries its line"
        );
    }
}
