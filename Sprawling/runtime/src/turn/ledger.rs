// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    Address, AxCode, AxError, EventDraft, EventKind, EventRef, Ledger, Payload, RunId, TimeMs,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum Authored {
    PromptAssembled,

    ModelCalled { at: TimeMs },
    CancelReceived,
    SteerReceived,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RunLine {
    PromptShapeCompared,

    CheckpointCommitted,

    SecretEgressBlocked,
}

impl RunLine {
    fn kind(self) -> EventKind {
        match self {
            RunLine::PromptShapeCompared => EventKind::PromptShapeCompared,
            RunLine::CheckpointCommitted => EventKind::CheckpointCommitted,
            RunLine::SecretEgressBlocked => EventKind::SecretEgressBlocked,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Carried {
    ModelReturned { at: TimeMs },

    PrefixHeld { at: TimeMs },

    ToolCalled { at: TimeMs },

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
            Carried::PrefixHeld { .. } => EventKind::PrefixHeld,
            Carried::ToolCalled { .. } => EventKind::ToolCalled,
            Carried::ToolResult { .. } => EventKind::ToolResult,
        }
    }

    fn at(self) -> TimeMs {
        match self {
            Carried::ModelReturned { at }
            | Carried::PrefixHeld { at }
            | Carried::ToolCalled { at }
            | Carried::ToolResult { at } => at,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Moment {
    pub(super) at: TimeMs,
    pub(super) us: Option<u64>,
}

impl Moment {
    pub(super) fn since(self, earlier: Option<u64>) -> Option<u64> {
        Some(self.us?.saturating_sub(earlier?))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeldLines {
    run: RunId,
    who: String,
    held: Vec<EventDraft>,
    refs: Vec<EventRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry(usize);

impl HeldLines {
    pub fn open(run: RunId, who: String) -> HeldLines {
        HeldLines {
            run,
            who,
            held: Vec::new(),
            refs: Vec::new(),
        }
    }

    pub fn barrier(&mut self, ledger: &mut dyn Ledger) -> Result<(), AxError> {
        if self.held.is_empty() {
            return Ok(());
        }
        let echoes = ledger.append_all(std::mem::take(&mut self.held))?;
        self.refs.extend(echoes);
        Ok(())
    }

    pub fn refs(&self) -> &[EventRef] {
        &self.refs
    }

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

    pub(super) fn time_with(&mut self, monotonic_us: &'h mut dyn FnMut() -> u64) {
        self.stopwatch = Some(monotonic_us);
    }

    pub(super) fn redacted(&self) -> u32 {
        self.redacted
    }

    pub(super) fn lines(&mut self) -> &mut HeldLines {
        self.lines
    }

    pub(super) fn barrier(&mut self, ledger: &mut dyn Ledger) -> Result<(), AxError> {
        self.lines.barrier(ledger)
    }

    pub(super) fn append_authored(&mut self, event: Authored, data: Payload) -> Entry {
        let at = event.at(self.t);
        self.append(event.kind(), at, None, data)
    }

    pub(super) fn append_run_line(&mut self, line: RunLine, addr: Address, data: Payload) -> Entry {
        self.append(line.kind(), self.t, Some(addr), data)
    }

    pub(super) fn append_redacted(
        &mut self,
        event: Carried,
        data: Payload,
    ) -> Result<Entry, AxError> {
        let (scanned, hits) = crate::redact::redact(data.into_map());
        self.redacted = self.redacted.saturating_add(hits);
        Ok(self.append(event.kind(), event.at(), None, Payload::new(scanned)?))
    }

    pub(super) fn stamp(&self) -> TimeMs {
        self.t
    }

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
