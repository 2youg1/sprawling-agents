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
//! clock sample, the record it appends, the two hooks a live control
//! surface installs, and the door a `Command` enters by. Opening and
//! closing live in `lifetime`; the test fixtures in `fixture`.
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
mod reviewing;
mod settling;
mod waking;
mod workbench;

pub(crate) use building_page::read_building;
use credentials::{Ceilings, Chosen, Entered};
use dispatching::{Agreed, Assignment, DISPATCH_TURN_BUDGET, Given, Knock, run_id_for};
pub(crate) use dispatching::{Dispatched, acp_dispatch};
use driving::{Driven, Driving};
use folds::{Governance, HALTED, RELEASED, artifact_of, new_inbox};
pub(crate) use folds::{Standing, rebuild_views};
pub(crate) use genesis::city_address;
use genesis::city_segment;
pub use genesis::{Adopt, InitReport, form_city, has_history, init_city};
use mcp::{PYTHON_WASM_ENV, connect_mcp, mounts_under, transport_site};
use naming::{
    autonomy_name, building_of, mode_of, name_of, not_built, plan_node_of, read_autonomy,
    scope_name,
};
use plans::Reporter;
use settling::{Ending, Sweep};
use workbench::{CITY_VERIFIER, Desks, Site, Workbench};

use crate::effect;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError, EventDraft, EventKind};
use kernel::{EventRecord, Ledger, Payload, RunId, TimeMs};
use memory::{Cas, JsonlLedger};
use runtime::Interrupt;

use crate::serving::Posted;

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

/// Runs the work a Command asks for. It owns the ledger, so the city has
/// one writer; commands reach it through a channel, and the socket task
/// that accepted them is free again immediately.
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

pub struct RunWorker {
    city_root: PathBuf,
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
    interrupts: Option<Box<dyn FnMut(RunId) -> Interrupt + Send>>,
    /// Where a model's text goes while it is still arriving. `None` in
    /// every worker but the one behind a live control surface, and that
    /// is the switch: a run whose city has nobody watching asks its
    /// provider for no stream at all, so replay and citysim take the
    /// byte-identical path they always took.
    watching: Option<std::sync::Arc<dyn Fn(channels::Delta) + Send + Sync>>,
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
}

impl RunWorker {
    /// Writes one diagnostic line, anchored to where the ledger stands.
    fn note(&mut self, level: runtime::diagnostics::Level, module: &str, message: &str) {
        let site = runtime::diagnostics::Site {
            run: RunId::CITY,
            seq: self.ledger.position(),
            module,
        };
        self.log.write(level, site, message);
    }

    /// Appends one city record and folds it into the worker's own book.
    /// The append comes first: the book states what the history says,
    /// never what the process hoped to write.
    fn record(&mut self, kind: EventKind, data: Payload) -> Result<(), AxError> {
        self.record_where(kind, None, data)
    }

    /// The same, for a record that belongs to one address. A pursuit is
    /// a building's, and a record with no address would be a fact about
    /// the city that no view could file under the building it changed.
    fn record_at(&mut self, kind: EventKind, addr: Address, data: Payload) -> Result<(), AxError> {
        self.record_where(kind, Some(addr), data)
    }

    fn record_where(
        &mut self,
        kind: EventKind,
        addr: Option<Address>,
        data: Payload,
    ) -> Result<(), AxError> {
        let draft = EventDraft {
            run: RunId::CITY,
            t: now_ms()?,
            who: "owner".to_owned(),
            addr,
            kind,
            data: data.clone(),
            ig: false,
        };
        self.ledger.append(draft)?;
        self.governance.absorb(kind, RunId::CITY, None, &data);
        self.book.apply_payload(kind, &data)
    }

    /// Appends one line attributed to a run rather than to the city.
    /// Separate from `record` because that one speaks for the city: an
    /// effect a resident caused must carry the resident's name, or the
    /// history cannot say who spoke.
    fn record_for(&mut self, run: RunId, line: effect::Line) -> Result<(), AxError> {
        let effect::Line {
            who,
            addr,
            kind,
            data,
        } = line;
        self.ledger.append(EventDraft {
            run,
            t: now_ms()?,
            who,
            addr: Some(addr.clone()),
            kind,
            data: data.clone(),
            ig: false,
        })?;
        // The book states what the history says, whoever wrote the line.
        // Without this an approval a run raised was on the ledger and
        // absent from `pending`, so the person could not answer it until
        // the process restarted and folded the ledger again. It is the
        // same fold a restart runs, shown the line this process wrote.
        self.governance.absorb(kind, run, Some(&addr), &data);
        Ok(())
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

    /// Where a running dispatch asks what arrived. Attached by the serve
    /// wiring, absent in a worker driven command by command: a source
    /// nobody set means a run that nothing interrupts.
    pub(crate) fn attach_interrupts(&mut self, source: Box<dyn FnMut(RunId) -> Interrupt + Send>) {
        self.interrupts = Some(source);
    }

    /// # Errors
    /// Refuses a command this stage does not run yet, naming what does.
    pub fn handle(&mut self, command: channels::Command) -> Result<(), AxError> {
        let name = command.name();
        let outcome = self.run_command(command);
        if let Err(err) = &outcome {
            // A refused command is the first thing a person asks about,
            // so it is written at the default floor. It is written here
            // rather than at the caller, because every caller wants it.
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                &format!("{name} refused: {err}; {}", err.recovery()),
            );
        }
        outcome
    }

    /// Runs one command from the desk, refusal included.
    ///
    /// The one authority for what becomes of a command a person sent:
    /// it runs, and if it is refused the refusal goes both to the
    /// diagnostic log and to whoever asked. Before this existed the
    /// worker loop wrote `let _ = handle(command)`, so every refusal
    /// died in the log and the page that caused it said nothing.
    pub(crate) fn serve_one(&mut self, posted: Posted) {
        let Posted { command, reply } = posted;
        if let Err(err) = self.handle(command) {
            self.hand_back(&reply, err);
        }
    }

    /// Hands a refusal to whoever asked for the command.
    ///
    /// `handle` has already written it to the diagnostic log, so the
    /// only case that earns a second line is the one a reader would
    /// otherwise misread: somebody did ask, and the answer arrived at a
    /// socket that had already closed.
    fn hand_back(&mut self, reply: &channels::Reply, error: AxError) {
        match reply.refuse(error) {
            channels::Delivered::ToThePeer | channels::Delivered::NobodyAsked => {}
            channels::Delivered::PeerGone => self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                "the refusal above reached nobody: the peer that asked had closed its socket",
            ),
        }
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
