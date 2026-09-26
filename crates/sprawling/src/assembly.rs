// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Main's assembly point — the dirtiest component and the only
//! omniscient one: it knows every concrete type, and nothing knows it.
//! Ledger handle, clock source, RNG seed and spawn points are injected
//! from here and nowhere else; citysim is the second Main.
//!
//! The clock is sampled *here only* (determinism rule 2): every callee
//! takes time as a parameter, and the sample stays in this file so that
//! the rule keeps naming one place.
//!
//! **This file holds the worker and the modules below hold its methods.**
//! `RunWorker` is declared here, so its private fields are
//! visible throughout `assembly` and nowhere else — a private item
//! reaches the module that declares it and that module's descendants, so
//! the split cost no field its privacy. What stays here is what every
//! submodule needs and no submodule owns: the worker itself, the one
//! clock sample, the two hooks a live control surface installs, and the
//! door a `Command` enters by. The lines it appends live in
//! `recording`; opening and closing in `lifetime`; the test fixtures in
//! `fixture`.
//!
//! The `use` block below is where the submodules see each other. A
//! submodule imports from `super`, so what one part of the assembly
//! point offers another is stated once, here, and reads as a list rather
//! than as a graph; the one sibling reach is `credentials::dialect_headers`,
//! which the dispatching modules read where the credentials module keeps it.

mod building_page;
mod collaborating;
mod commanding;
mod credentials;
mod dispatching;
mod driving;
mod folds;
mod freezing;
mod genesis;
mod lifetime;
mod mcp;
mod naming;
mod plans;
mod probing;
mod recording;
mod reviewing;
mod rooms;
mod settling;
mod toolkits;
mod waking;
mod workbench;

pub(crate) use building_page::{DOC_BYTES_MAX, read_building};
use collaborating::Collaborating;
use commanding::entrance::Entrance;
use credentials::held::Credentials;
pub(crate) use credentials::signing::resolving;
use credentials::subscription::Expiries;
use credentials::{Ceilings, Chosen, Credential, Entered, tuning_of};
use dispatching::asking_name::Namings;
use dispatching::running::Continuation;
use dispatching::{Agreed, Assignment, Given, Handover, Knock, run_id_for};
pub(crate) use dispatching::{Dispatched, acp_dispatch};
use driving::flight::{Flight, Landed};
pub(crate) use driving::lane::{DriveContext, drive_run};
use driving::owing::{Owed, Owing, Unasked};
pub(crate) use driving::{Driven, Driving};
use folds::{Governance, INBOX_CAPACITY, SessionOrigins, new_inbox};
pub(crate) use folds::{Standing, rebuild_views};
use genesis::city_segment;
pub use genesis::{Adopt, History, InitReport, form_city, has_history, init_city};
pub(crate) use mcp::McpLink;
use mcp::{connect_mcp, mounts_under, transport_site};
use naming::{building_of, governed_of, name_of, not_built, scope_of};
use plans::Reporter;
use plans::held::{PlanHolders, Planning};
use rooms::{QueueTenure, RoomQueues};
use settling::{Ending, Settling, Sweep};
pub(crate) use toolkits::broker_for;
use workbench::{CITY_VERIFIER, Desks, Site, Workbench, held};

use std::path::PathBuf;
use std::sync::Arc;

use kernel::{AxCode, AxError, EventRecord, RunId, TimeMs};
// What the test fixtures below reach through `super::*`, now that the
// lines this worker appends live in `recording`.
#[cfg(test)]
use crate::effect;
#[cfg(test)]
use kernel::{Address, EventDraft, EventKind, Payload};
use memory::{Cas, JsonlLedger};
use runtime::Interrupt;
#[cfg(test)]
use std::path::Path;

/// The single sanctioned sampling point (clippy.toml disallowed-methods). Everything below this call takes `TimeMs` as a
/// parameter.
pub(crate) fn now_ms() -> Result<TimeMs, AxError> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the one sampling point: Main injects time"
    )]
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "sample wall clock", err.to_string())
                .with_recovery("fix the system clock; it reads before the unix epoch")
        })?;
    let millis = u64::try_from(elapsed.as_millis()).map_err(|_| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "sample wall clock",
            "beyond u64 millis",
        )
        .with_recovery(
            "set this machine's clock to the present day; it reads more than half a \
             billion years after the unix epoch",
        )
    })?;
    Ok(TimeMs::new(millis))
}

/// What the startup scan found and repaired.
pub struct ScanReport {
    pub(crate) lines: usize,
    pub(crate) closed_calls: usize,
    /// The one count a caller branches on rather than prints: `resume`
    /// adds a line telling the person where to answer. `lines` and
    /// `closed_calls` reach nobody outside `summary`, so they stay in.
    pub waiting_approvals: usize,
}

impl ScanReport {
    /// One line a person reads: what was verified, what was closed, and
    /// what is still owed an answer.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} line(s) verified; {} unknown-outcome call(s) closed; {} approval(s) waiting",
            self.lines, self.closed_calls, self.waiting_approvals
        )
    }
}

/// Where a served city listens, installed once by whoever serves it.
///
/// The three sinks are one fact — *somebody is watching this city* —
/// and a worker that has any of them has all of them. A worker driven
/// one command at a time has none, and that absence is the switch: its
/// runs ask their provider for no stream at all, so replay and citysim
/// take the byte-identical path they always took.
pub(crate) struct Serving {
    /// Where a model's text goes while it is still arriving.
    pub(crate) deltas: Arc<dyn Fn(channels::Delta) + Send + Sync>,
    /// Where a fresh look at this machine goes: the one place the
    /// doctor's answer is replaced after the look taken at start-up.
    pub(crate) machine: Arc<dyn Fn(channels::DoctorAnswer) + Send + Sync>,
    /// What a running dispatch asks at its safe points.
    ///
    /// One handle per drive rather than one hook lent out and taken
    /// back: N runs may be asking at once, and each asks about itself
    /// (sprawling-SPEC.md 8-46-1).
    pub(crate) interrupts: Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
}

/// Runs the work a Command asks for. It owns the ledger, so the city has
/// one writer; commands reach it through a desk, and the socket task
/// that accepted them is free again immediately.
pub struct RunWorker {
    city_root: PathBuf,
    /// Which city this is, as the genesis line hashes. Read from the
    /// ledger the first time a commit needs signing and remembered:
    /// every fence, landing and merge used to re-read the front of the
    /// history for it (sprawling-SPEC.md 8-51). Lazy rather than read on
    /// open, because a worker over a city with no genesis line yet is a
    /// legal state.
    city: std::sync::OnceLock<kernel::B3Hash>,
    ledger: JsonlLedger,
    cas: Cas,
    /// Whose identity this city can call which model under
    /// (`credentials::held`).
    credentials: Credentials,
    /// The three places a live control surface listens, or `None` in a
    /// worker driven one command at a time.
    ///
    /// **One `Option`, not three.** The three sinks are installed by
    /// one caller in one breath and are absent together in every other
    /// worker; as three fields the type admitted eight states of which
    /// two were reachable, and adding a fourth sink meant remembering a
    /// fourth setter (sprawling-SPEC.md 8-46-10).
    serving: Option<Serving>,
    /// What waits for a person, who may answer it, what has been
    /// allowed, which scopes are shut, and what each waiting item is
    /// holding up. The worker keeps its own copy for the same reason it
    /// keeps the endpoint book: an answer is decided synchronously,
    /// before the record it just wrote has reached any observer.
    governance: Governance,
    /// What residents are handing one another (`collaborating`).
    collaborating: Collaborating,
    /// What each building is working towards and who holds which part
    /// of its plan (`plans::held`).
    planning: Planning,
    /// The instant the schedule was last read against. Set when the
    /// worker opens, so a city that was off owes nothing for the time it
    /// was off.
    last_tick: TimeMs,
    /// The diagnostic log. Write-only, and nothing here reads it back:
    /// turning it off must leave the ledger byte-identical.
    log: runtime::diagnostics::Diagnostics,
    /// Residents who were spoken to while nobody was home. Held between
    /// the run that spoke and the runs that answer, because delivery
    /// happens after the speaker has frozen.
    knocks: Vec<Knock>,
    /// Every command key this city has answered, and what it answered.
    /// Folded from the history like the endpoint book beside it, so a
    /// client retrying across a restart is still asking for one thing.
    entrance: Entrance,
    /// What is still running while the runs go on. One table per city,
    /// and every `exec` gets a handle onto it, so `halt` reaches a
    /// command without knowing which tool started it.
    backlog: runtime::Backlog,
    /// What each room's current session branched from, until the run
    /// that begins it is written (`assembly::folds::session`).
    pub(in crate::assembly) origins: SessionOrigins,
    /// One fence at a time per city: a repository has one index, and
    /// every lane of this worker stages and commits it (`driving::lane`).
    pub(in crate::assembly) fence_gate: std::sync::Arc<std::sync::Mutex<()>>,
    /// Every run in a lane right now, the crossing those lanes write
    /// history through, and what the city owes each one when it comes
    /// home. One per city, so the number of runs a city drives at once
    /// has one answer (sprawling-SPEC.md 8-46-2).
    flight: Flight,
    /// Dispatches waiting on the digest model for a room name, off this
    /// thread (sprawling-SPEC.md 8-86).
    namings: Namings,
}

impl RunWorker {
    /// Which city this is, read from the genesis line once and
    /// remembered for the life of the worker.
    ///
    /// One read serves every commit this city makes. The value cannot
    /// change without the city being a different city, so remembering it
    /// removes a re-read rather than creating a second authority - and a
    /// re-read that answered differently mid-run would be the worse
    /// failure of the two (sprawling-SPEC.md 8-51).
    ///
    /// # Errors
    /// Propagates a ledger that cannot be read and a city with no
    /// genesis line: a city with no genesis has no identity to sign
    /// with.
    pub(crate) fn city_hash(&self) -> Result<kernel::B3Hash, AxError> {
        if let Some(known) = self.city.get() {
            return Ok(*known);
        }
        let read =
            memory::Provenance::city_of(&kernel::layout::CityLayout::new(&self.city_root).ledger())
                .map_err(memory::MemoryError::into_ax)?;
        Ok(*self.city.get_or_init(|| read))
    }

    /// Sends every appended record to `sink` once it is durable.
    pub(crate) fn observe(&mut self, sink: Box<dyn FnMut(&EventRecord) + Send>) {
        self.ledger.observe(sink);
    }

    /// Takes the three places a served city listens.
    ///
    /// Separate from [`Self::observe`] because the two carry different
    /// kinds of thing: that one carries history, and these carry a view
    /// of work in progress and of the machine under it. One call rather
    /// than three, so a worker cannot end up streaming to a page that
    /// cannot interrupt it.
    pub(crate) fn serve(&mut self, serving: Serving) {
        self.serving = Some(serving);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
pub(super) mod fixture;
