// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A run whose turn an official harness takes: every line the city
//! writes for it, and the completion its answer freezes as
//! (runtime-SPEC.md 8-52).
//!
//! The harness runs its own tools, so the city writes what it decided -
//! the opening, the cancel, the tree it committed, the answer to its own
//! prompt, the freeze - and books what the harness reported as it went.
//! The order of those lines and the ending are proved in
//! `crates/agent_protocols/spec/Harness/Session.lean`; the types here
//! hold the order, because `conclude` and `abandon` take the run by value
//! and a frozen run has nothing left to write with.

use kernel::event::record::{CancelReceived, HarnessAnswered, HarnessReported, HarnessStop};
use kernel::{AxError, Completion, EventKind, EventRef, Evidence, Ledger, Payload, TimeMs};

use crate::handoff::Handoff;

use super::charter::Charter;

/// Why the city cut a harness's turn short.
///
/// Both become the one `cancel_received` and the one `session/cancel`;
/// they part only at the freeze, because a run the ceiling stopped hit
/// something and a run a halt stopped was stopped by somebody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    /// A halt of a scope that holds the room; a person's cancel of the
    /// run is one.
    Halt,
    /// The wall-clock ceiling the building's rules set.
    Deadline,
}

/// What a harness's answer leaves for the run to write: the tree as it
/// stood when the harness stopped, what it answered, and the handoff.
pub struct Conclusion<'c> {
    /// The `checkpoint_committed` payload `storage::Checkpoint::wave_pre`
    /// handed back for the harness's tree.
    pub committed: Payload,
    pub answered: HarnessAnswered,
    pub handoff: &'c Handoff,
}

/// A harness run between its opening and its freeze.
pub struct HarnessRun<'a> {
    charter: Charter<'a>,
    /// The first cut the city made, which the ending reads.
    cut: Option<Cut>,
}

impl<'a> HarnessRun<'a> {
    /// Writes the dispatch pair, as a model run's dispatch does.
    ///
    /// # Errors
    /// Propagates the clock and the ledger.
    pub fn open(
        charter: Charter<'a>,
        ledger: &mut dyn Ledger,
        now: &mut dyn FnMut() -> Result<TimeMs, AxError>,
    ) -> Result<HarnessRun<'a>, AxError> {
        charter.open(ledger, now)?;
        Ok(HarnessRun { charter, cut: None })
    }

    /// Books one thing the harness reported, after it said it.
    ///
    /// # Errors
    /// Propagates a payload that will not encode and the ledger.
    pub fn report(
        &self,
        ledger: &mut dyn Ledger,
        reported: &HarnessReported,
        t: TimeMs,
    ) -> Result<(), AxError> {
        ledger.append(
            self.charter
                .line(EventKind::HarnessReported, Payload::of(reported)?, t),
        )?;
        Ok(())
    }

    /// Books the cancel the city is about to send, the first time the
    /// turn is cut; a later cut writes nothing and the first one is the
    /// one the ending reads.
    ///
    /// # Errors
    /// Propagates the ledger.
    pub fn cancel(&mut self, ledger: &mut dyn Ledger, cut: Cut, t: TimeMs) -> Result<(), AxError> {
        if self.cut.is_some() {
            return Ok(());
        }
        ledger.append(self.charter.line(
            EventKind::CancelReceived,
            Payload::of(&CancelReceived {})?,
            t,
        ))?;
        self.cut = Some(cut);
        Ok(())
    }

    /// The harness answered: the tree, then the answer, then the freeze,
    /// and the completion the answer and the first cut decide.
    ///
    /// # Errors
    /// Propagates the ledger, and refuses a `t` at the end of `u64`.
    pub fn conclude(
        self,
        ledger: &mut dyn Ledger,
        conclusion: Conclusion<'_>,
        t: TimeMs,
    ) -> Result<Completion, AxError> {
        let Conclusion {
            committed,
            answered,
            handoff,
        } = conclusion;
        ledger.append(
            self.charter
                .line(EventKind::CheckpointCommitted, committed, t),
        )?;
        let cited = ledger.append(self.charter.line(
            EventKind::HarnessAnswered,
            Payload::of(&answered)?,
            t,
        ))?;
        let completion = ending(&answered, self.cut, cited)?;
        self.charter.close(ledger, handoff, &completion, t)?;
        Ok(completion)
    }

    /// The session ended with no answer: there is nothing to record as
    /// one, so the run freezes cancelled.
    ///
    /// # Errors
    /// Propagates the ledger, and refuses a `t` at the end of `u64`.
    pub fn abandon(
        self,
        ledger: &mut dyn Ledger,
        handoff: &Handoff,
        t: TimeMs,
    ) -> Result<Completion, AxError> {
        self.charter
            .close(ledger, handoff, &Completion::Cancelled, t)?;
        Ok(Completion::Cancelled)
    }
}

/// How an answer ends the run: `Session.lean`'s `ending`.
///
/// An answer with nothing in it is not evidence that work finished, the
/// reading `lifecycle::concluded` gives an empty model reply.
fn ending(
    answered: &HarnessAnswered,
    cut: Option<Cut>,
    cited: EventRef,
) -> Result<Completion, AxError> {
    Ok(match answered.stop {
        HarnessStop::EndTurn if answered.text.trim().is_empty() => Completion::Limit,
        HarnessStop::EndTurn => Completion::Done(Evidence::new(vec![cited])?),
        HarnessStop::Cancelled => match cut {
            Some(Cut::Deadline) => Completion::Limit,
            Some(Cut::Halt) | None => Completion::Cancelled,
        },
        HarnessStop::MaxTokens | HarnessStop::MaxTurnRequests | HarnessStop::Refusal => {
            Completion::Limit
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
