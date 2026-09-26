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

use super::{
    Collaborating, Credentials, Doorstep, Flight, GatewayModels, McpServers, Planning, RoomQueues,
    RunWorker, Standing, SystemClock, city_segment,
};
use crate::doctor::{PATIENCE, Platform, ThisMachine};
use std::path::Path;

use kernel::{AxError, EventKind, Locator};
use memory::{Cas, JsonlLedger};

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
        let dir = kernel::layout::CityLayout::new(city_root).ledger();
        let (ledger, _report) = JsonlLedger::open(&dir, accounting::Clock::now(&SystemClock)?)
            .map_err(memory::MemoryError::into_ax)?;
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
        let now = accounting::Clock::now(&SystemClock)?;
        let dir = kernel::layout::CityLayout::new(city_root).ledger();
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
            credentials: Credentials::opened(book, expiries, vault),
            serving: None,
            governance,
            collaborating: Collaborating {
                rooms: RoomQueues::folded(collaboration.inboxes),
                joins: collaboration.joins,
                requests: collaboration.requests,
                goals: collaboration.goals,
            },
            planning: Planning {
                pursuits,
                delegator,
                holders: collaboration.plan_holders,
            },
            last_tick: now,
            log,
            doorstep: Doorstep::opened(entrance),
            origins,
            flight: Flight::open(),
            models: Box::new(GatewayModels),
            connectors: Box::new(McpServers),
            machine: Box::new(ThisMachine::new(Platform::current(), PATIENCE)),
            clock: std::sync::Arc::new(SystemClock),
        })
    }

    /// Closes the city in the record, so a stop somebody chose and a
    /// stop that was a crash are different lines rather than the same
    /// silence.
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
    pub(crate) fn close_city(&mut self) -> Result<(), AxError> {
        // The city's own norm, not a building's: `city::norms` answers
        // for a run at an address, and this line belongs to the city.
        // Through the same reader the prefix uses. What this city's
        // norms are has one answer, and hashing zero bytes here made the
        // must-read locator point at nothing while the handoff went on
        // saying the next session must read them.
        let mut must_read = Vec::new();
        let bytes = city_segment(&self.city_root)?.bytes;
        let hash = self.cas.put(&bytes).map_err(memory::MemoryError::into_ax)?;
        must_read.push(Locator::cas(hash));
        let standing = self.ledger.position();
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            "the city was closed by the person running it".to_owned(),
            format!("the ledger stands at {}", standing.value()),
            "an orderly close, not a crash: nothing was interrupted mid-command".to_owned(),
            "`sprawling serve` on this directory continues from here".to_owned(),
        )?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "bin::assembly",
            "the city is closing; its handoff is on the ledger",
        );
        self.record(EventKind::HandoffWritten, handoff.payload()?)
    }

    /// The same worker, reading every time through `clock` instead of
    /// the wall clock (accounting-SPEC.md 8-3).
    ///
    /// The door citysim and the tests drive a worker through: when
    /// things happen is theirs to script, while what the worker writes
    /// at those times stays its own. The ledger was opened before this
    /// door, at the wall clock's time.
    #[must_use]
    pub fn with_clock(
        self,
        clock: std::sync::Arc<dyn accounting::Clock + Send + Sync>,
    ) -> RunWorker {
        RunWorker { clock, ..self }
    }
}
