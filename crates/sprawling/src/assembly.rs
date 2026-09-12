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
//! `RunWorker` is declared here, so its twenty-two private fields are
//! visible throughout `assembly` and nowhere else — a private item
//! reaches the module that declares it and that module's descendants, so
//! the split cost no field its privacy. What stays here is what every
//! submodule needs and no submodule owns: the worker itself, the one
//! clock sample, the two hooks a live control surface installs, and the
//! door a `Command` enters by. The lines it appends live in
//! `recording`; opening and closing in `lifetime`; the test fixtures in
//! `fixture`.
//!
//! The `use` block below is the one place the sixteen submodules see
//! each other through. A submodule imports from `super`, never from a
//! sibling, so what one part of the assembly point offers another is
//! stated once, here, and reads as a list rather than as a graph.

mod building_page;
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
mod settling;
mod waking;
mod workbench;

pub(crate) use building_page::{DOC_BYTES_MAX, read_building};
use commanding::entrance::Entrance;
pub(crate) use credentials::signing::resolving;
use credentials::{Ceilings, Chosen, Credential, Entered, tuning_of};
use dispatching::running::Continuation;
use dispatching::{Agreed, Assignment, Given, Handover, Knock, run_id_for};
pub(crate) use dispatching::{Dispatched, acp_dispatch};
pub(crate) use driving::flight::LOOK_AGAIN;
use driving::flight::{Flight, Landed};
pub(crate) use driving::lane::{DriveContext, drive_run};
pub(crate) use driving::{Driven, Driving};
use folds::{Governance, artifact_of, new_inbox};
pub(crate) use folds::{HALTED, RELEASED, Standing, rebuild_views};
pub(crate) use genesis::city_address;
use genesis::city_segment;
pub use genesis::{Adopt, InitReport, form_city, has_history, init_city};
pub(crate) use mcp::McpLink;
use mcp::{connect_mcp, mounts_under, transport_site};
pub(crate) use naming::read_autonomy;
use naming::{
    autonomy_name, building_of, governed_of, mode_of, name_of, not_built, plan_node_of, scope_name,
};
use plans::Reporter;
use settling::{Ending, Sweep};
use workbench::{CITY_VERIFIER, Desks, Site, Workbench, held};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError, EventRecord, RunId, TimeMs};
// What the test fixtures below reach through `super::*`, now that the
// lines this worker appends live in `recording`.
#[cfg(test)]
use crate::effect;
#[cfg(test)]
use kernel::{EventDraft, EventKind, Payload};
use memory::{Cas, JsonlLedger};
use runtime::Interrupt;

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
    })?;
    Ok(TimeMs::new(millis))
}

/// Where a city keeps its ledger: under the reserved prefix, outside
/// every WriteDomain (C17).
pub(crate) fn ledger_dir(city_root: &Path) -> PathBuf {
    city_root.join(".sprawling").join("ledger")
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
    /// Every endpoint the person attached and every model they chose,
    /// folded from the ledger. The worker keeps its own copy because a
    /// dispatch needs it synchronously, before the record it just wrote
    /// has reached any observer.
    book: gateway::EndpointBook,
    /// The vault. Shared because a redemption closure outlives the call
    /// that builds it; the lock is held for one resolve at a time.
    vault: Arc<std::sync::Mutex<gateway::Custodian>>,
    /// What a running dispatch asks at its safe points. `None` in a
    /// worker driven one command at a time, which is every worker except
    /// the one behind a live control surface.
    /// One handle per drive rather than one hook lent out and taken
    /// back: N runs may be asking at once, and each asks about itself
    /// (sprawling-SPEC.md 8-46-1).
    interrupts: Option<Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    /// Where a model's text goes while it is still arriving. `None` in
    /// every worker but the one behind a live control surface, and that
    /// is the switch: a run whose city has nobody watching asks its
    /// provider for no stream at all, so replay and citysim take the
    /// byte-identical path they always took.
    watching: Option<std::sync::Arc<dyn Fn(channels::Delta) + Send + Sync>>,
    /// Where a fresh look at this machine goes. `None` in a worker
    /// driven one command at a time, which has no views to correct;
    /// where a city is served it is the one place the doctor's answer
    /// is replaced after the look taken at start-up.
    machine: Option<std::sync::Arc<dyn Fn(channels::DoctorAnswer) + Send + Sync>>,
    /// What waits for a person, who may answer it, what has been
    /// allowed, which scopes are shut, and what each waiting item is
    /// holding up. The worker keeps its own copy for the same reason it
    /// keeps the endpoint book: an answer is decided synchronously,
    /// before the record it just wrote has reached any observer.
    governance: Governance,
    /// What is waiting for each room, folded from the signal records.
    /// A dispatch lends its room's queue to the signal tool and takes it
    /// back when the drive ends, so exactly one queue exists per room.
    pub(super) inboxes: std::collections::BTreeMap<Address, collab::Inbox>,
    /// What each room already got back from work it handed down. Kept
    /// beside the inboxes because it is folded from the same lines and
    /// belongs to the same room.
    pub(super) joins: std::collections::BTreeMap<Address, collab::FanIn>,
    /// The requests waiting for someone to check them, folded from the
    /// pull request records.
    pub(super) requests: Vec<collab::OpenRequest>,
    /// The ground residents have claimed, folded from `goal_registered`
    /// in the order the claims were made — which is the order the
    /// conflict check reads them in.
    pub(super) goals: Vec<kernel::GoalEntry>,
    /// What each building is working towards, and the depth-zero
    /// position that lets one be declared. Held by the worker because
    /// the worker is what acts on it; rebuilt from the records on open,
    /// like the endpoint book and the goal register beside it.
    pursuits: std::collections::BTreeMap<Address, kernel::Pursuit>,
    delegator: kernel::Delegator,
    /// Which room holds each node of each building's plan, folded from
    /// the claim records and kept up to date as this worker writes them.
    plan_holders:
        std::collections::BTreeMap<Address, std::collections::BTreeMap<kernel::NodeId, String>>,
    /// The instant the schedule was last read against. Set when the
    /// worker opens, so a city that was off owes nothing for the time it
    /// was off.
    last_tick: TimeMs,
    /// Whether the run being dispatched began with somebody else's text.
    /// Set for the length of one dispatch by `wake`; it decides whether
    /// the approvals that run raises can be waived by a policy.
    tainted_arrival: bool,
    /// When each subscription credential stops working, by provider.
    /// Folded from the capture records, so a restarted city renews on
    /// the same schedule rather than discovering expiry through a 401.
    expiries: std::collections::BTreeMap<String, u64>,
    /// Logins begun and not yet redeemed, by provider. Held in memory
    /// on purpose: a PKCE verifier proves that the process which asked
    /// is the process which redeems, so a verifier that outlived the
    /// process would be proving nothing. A restart means starting the
    /// login again, which is one browser visit.
    logins: std::collections::BTreeMap<String, gateway::OauthPending>,
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
    /// Every run in a lane right now, the crossing those lanes write
    /// history through, and what the city owes each one when it comes
    /// home. One per city, so the number of runs a city drives at once
    /// has one answer (sprawling-SPEC.md 8-46-2).
    flight: Flight,
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
        let read = memory::Provenance::city_of(&ledger_dir(&self.city_root))
            .map_err(memory::MemoryError::into_ax)?;
        Ok(*self.city.get_or_init(|| read))
    }

    /// Sends every appended record to `sink` once it is durable.
    pub(crate) fn observe(&mut self, sink: Box<dyn FnMut(&EventRecord) + Send>) {
        self.ledger.observe(sink);
    }

    /// Sends every increment a model produces to `sink`, before the call
    /// it belongs to has settled.
    ///
    /// Separate from [`Self::observe`] because the two carry different
    /// kinds of thing: that one carries history and this one carries a
    /// view of work in progress. A city with nobody watching never
    /// installs one, and a run whose city has none asks its provider for
    /// no stream at all.
    pub(crate) fn watch(&mut self, sink: std::sync::Arc<dyn Fn(channels::Delta) + Send + Sync>) {
        self.watching = Some(sink);
    }

    /// Sends a fresh look at this machine to `sink`.
    ///
    /// Separate from [`Self::observe`] because what travels is not
    /// history: the doctor's answer is about the machine rather than
    /// about the city, which is why it is set from outside and why a
    /// rebuild from the ledger leaves it alone.
    pub(crate) fn examine(
        &mut self,
        sink: std::sync::Arc<dyn Fn(channels::DoctorAnswer) + Send + Sync>,
    ) {
        self.machine = Some(sink);
    }

    /// Where a running dispatch asks what arrived. Attached by the serve
    /// wiring, absent in a worker driven command by command: a source
    /// nobody set means a run that nothing interrupts.
    pub(crate) fn attach_interrupts(
        &mut self,
        source: Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
    ) {
        self.interrupts = Some(source);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
pub(super) mod fixture;
