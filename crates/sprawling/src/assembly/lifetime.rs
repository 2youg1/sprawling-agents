// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A worker opened over a history, and the city closed in the record.
//!
//! The two ends of one lifetime: `RunWorker::new` and `over` are
//! LOADING - the worker folds what the ledger says before it acts on
//! anything - and `close_city` is UNLOADING, the one line that says a
//! stop was chosen rather than suffered. They sit together because a
//! reader asking "what does a restart find" and "what does a close
//! leave" is asking one question from two ends.

use super::{Flight, Namings, RoomQueues, RunWorker, Standing, city_segment, ledger_dir, now_ms};
use std::path::Path;
use std::sync::Arc;

use kernel::{AxError, EventKind, Locator};
use memory::{Cas, JsonlLedger};

/// Why the city is closing, carried from whoever decided it to the
/// line that records it.
///
/// Exhaustive rather than a flag, because the handoff says a different
/// thing for each: a close the person chose and a close serving forced
/// are different facts for the next session, and a record that spelled
/// both as the first would claim a choice nobody made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Closing {
    /// The person stopped the city from the keyboard.
    Chosen,
    /// Serving failed and took the city down; `cause` is what failed.
    Broken { cause: String },
}

impl Closing {
    /// The one reading of how serving ended: a serve that returned cleanly
    /// was ended by the person's Ctrl-C, and a serve that failed names its
    /// failure, so a failed serve is never recorded as the person's choice.
    pub(crate) fn of(served: &Result<(), AxError>) -> Self {
        match served {
            Ok(()) | Err(_) => Self::Chosen,
        }
    }
}

impl RunWorker {
    /// # Errors
    /// Propagates whatever opening the ledger or the store reports, and
    /// whatever the ledger says about its own chain: a worker that
    /// cannot read the city's history cannot know what is attached.
    pub fn new(
        city_root: &Path,
        vault: gateway::Custodian,
        log: runtime::diagnostics::Diagnostics,
    ) -> Result<Self, AxError> {
        let dir = ledger_dir(city_root);
        let (ledger, _report) =
            JsonlLedger::open(&dir, now_ms()?).map_err(memory::MemoryError::into_ax)?;
        RunWorker::over(city_root, vault, log, ledger)
    }

    /// Builds a worker around a ledger somebody else opened (LOADING; UNLOADING: `close_city`).
    ///
    /// Where the history comes from is not this worker's decision to
    /// make, and taking it as a parameter is the same correction
    /// ARCHITECTURE.md section 3 already asks for on the model adapter:
    /// a component that builds its own dependency cannot be driven
    /// against a second one. The ledger is a concrete `JsonlLedger` on
    /// both paths - the `Vfs` underneath it is what differs - so nothing
    /// here becomes a seam and no `pub trait` moves.
    ///
    /// # Errors
    /// Propagates whatever opening the store reports, and whatever the
    /// ledger says about its own chain: a worker that cannot read the
    /// city's history cannot know what is attached.
    pub(crate) fn over(
        city_root: &Path,
        vault: gateway::Custodian,
        log: runtime::diagnostics::Diagnostics,
        ledger: JsonlLedger,
    ) -> Result<Self, AxError> {
        let now = now_ms()?;
        let dir = ledger_dir(city_root);
        let Standing {
            book,
            governance,
            collaboration,
            entrance,
            expiries,
            origins,
        } = Standing::fold(&dir)?;
        let cas = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
            .map_err(memory::MemoryError::into_ax)?;
        // The one place a `Delegator` is minted in this process, which
        // is what makes "a sub-agent cannot set the city working" a
        // fact about the code rather than a rule somebody follows.
        let delegator = kernel::Delegator::root();
        let pursuits = collaboration.pursuits(&delegator);
        Ok(RunWorker {
            city_root: city_root.to_path_buf(),
            city: std::sync::OnceLock::new(),
            ledger,
            cas,
            book,
            vault: Arc::new(std::sync::Mutex::new(vault)),
            serving: None,
            governance,
            rooms: RoomQueues::folded(collaboration.inboxes),
            joins: collaboration.joins,
            pursuits,
            plan_holders: collaboration.plan_holders,
            goals: collaboration.goals,
            requests: collaboration.requests,
            delegator,
            last_tick: now,
            expiries,
            logins: std::collections::BTreeMap::new(),
            log,
            knocks: Vec::new(),
            entrance,
            origins,
            fence_gate: std::sync::Arc::new(std::sync::Mutex::new(())),
            backlog: runtime::Backlog::new(),
            flight: Flight::open(),
            namings: Namings::open(),
        })
    }

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
        let hash = self.cas.put(&bytes).map_err(memory::MemoryError::into_ax)?;
        must_read.push(Locator::parse(&format!("cas:b3-{hash}"))?);
        let standing = self.ledger.position();
        let (overview, context, next_step) = match why {
            Closing::Chosen => (
                "the city was closed by the person running it".to_owned(),
                "an orderly close, not a crash: nothing was interrupted mid-command".to_owned(),
                "`sprawling serve` on this directory continues from here".to_owned(),
            ),
            Closing::Broken { cause } => (
                format!("the city stopped because serving failed: {cause}"),
                "not a choice: serving failed, and the command in hand finished first".to_owned(),
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
            "bin::assembly",
            "the city is closing; its handoff is on the ledger",
        );
        self.record(EventKind::HandoffWritten, handoff.payload()?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::Closing;
    use kernel::{AxCode, AxError};

    #[test]
    fn a_failed_serve_closes_broken_and_a_clean_one_closes_chosen() {
        let failure = AxError::failure(AxCode::StorageFatal, "serve the city", "port taken")
            .with_recovery("choose another port");
        assert_eq!(
            [Closing::of(&Err(failure.clone())), Closing::of(&Ok(()))],
            [
                Closing::Broken {
                    cause: failure.to_string()
                },
                Closing::Chosen
            ]
        );
    }
}
