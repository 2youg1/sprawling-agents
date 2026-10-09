// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Why the city is closing, and the line that records it: the UNLOADING
//! end of one worker's lifetime (`crates/accounting/spec/Worker.lean` §8-11).
//!
//! It sits beside [`super`], which holds the LOADING end, because a
//! reader asking "what does a restart find" and "what does a close
//! leave" is asking one question from two ends.

use super::super::{RunWorker, city_segment};
use kernel::{AxError, EventKind, Locator};

/// Why the city is closing, carried from whoever decided it to the
/// line that records it.
///
/// Exhaustive rather than a flag, because the handoff says a different
/// thing for each: a close the person chose and a close serving forced
/// are different facts for the next session, and a record that spelled
/// both as the first would claim a choice nobody made. Which event
/// becomes which closing is decided by the console's lifecycle
/// (`crates/sprawling/spec/Console/Lifecycle.lean`); this type only
/// carries the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closing {
    /// Somebody or something asked the city to close: who, and what
    /// becomes of the runs under way.
    Chosen { by: ClosedBy, mode: wire::CloseMode },
    /// Serving failed and took the city down; `cause` is what failed.
    Broken { cause: String },
}

/// Who asked the city to close.
///
/// A service manager's `SIGTERM` and a person's `/quit` are different
/// facts for whoever reads the handoff next, so each way in is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedBy {
    /// `/quit` typed in the city's own terminal.
    Console,
    /// `CloseCity` sent from a page on this machine.
    Page,
    /// `SIGINT` or `CTRL_C_EVENT`, as a signal, to a city with no
    /// terminal of its own.
    InterruptSignal,
    /// Windows' Ctrl+Break.
    BreakSignal,
    /// `SIGTERM`.
    Terminate,
    /// The terminal the city ran in went away.
    TerminalLost,
}

impl ClosedBy {
    /// Who closed it, as the handoff's overview says it.
    fn said(self) -> &'static str {
        match self {
            Self::Console => "with /quit in its terminal",
            Self::Page => "from a page on this machine",
            Self::InterruptSignal => "by an interrupt signal",
            Self::BreakSignal => "by Ctrl+Break",
            Self::Terminate => "by a terminate signal",
            Self::TerminalLost => "because its terminal went away",
        }
    }
}

impl RunWorker {
    /// Closes the city in the record, so a stop somebody chose, a stop
    /// serving forced, and a crash are three different records rather
    /// than one line and one silence.
    ///
    /// The five sections are the city's own: what the next session must
    /// read is the city's norms, and where it left off is the position
    /// the ledger stands at. Written through `runtime::handoff`, which
    /// is the one construction point for the shape - a hand-built
    /// payload here would be a second one.
    ///
    /// # Errors
    /// Propagates the handoff's refusal of an empty must-read list, and
    /// the ledger's refusal to take the line.
    pub(crate) fn close_city(&mut self, why: &Closing) -> Result<(), AxError> {
        // The city's own norm, not a building's: `city::norms` answers
        // for a run at an address, and this line belongs to the city.
        // Through the same reader the prefix uses. What this city's
        // norms are has one answer, and hashing zero bytes here made the
        // must-read locator point at nothing while the handoff went on
        // saying the next session must read them.
        let mut must_read = Vec::new();
        let bytes = city_segment(&self.city_root)?.bytes;
        let hash = self
            .cas
            .put(&bytes)
            .map_err(storage::StorageError::into_ax)?;
        must_read.push(Locator::cas(hash));
        let standing = self.ledger.position();
        let (overview, context, next_step) = match why {
            Closing::Chosen { by, mode } => (
                format!("the city was closed {}", by.said()),
                match mode {
                    wire::CloseMode::Drain => {
                        "an orderly close, not a crash: every run under way landed before this line"
                    }
                    wire::CloseMode::Interrupt => {
                        "an interrupted close, not a crash: runs still under way were stopped at their next safe point"
                    }
                }
                .to_owned(),
                "`sprawling serve` on this directory continues from here".to_owned(),
            ),
            Closing::Broken { cause } => (
                format!("the city stopped because serving failed: {cause}"),
                "not a choice: serving failed; the ledger holds every line that landed before it"
                    .to_owned(),
                "fix what the failure names, then `sprawling serve` on this directory".to_owned(),
            ),
        };
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            overview,
            format!("the ledger stands at {}", standing.value()),
            context,
            next_step,
        )?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "accounting::worker",
            "the city is closing; its handoff is on the ledger",
        );
        self.record(EventKind::HandoffWritten, handoff.payload()?)
    }
}
