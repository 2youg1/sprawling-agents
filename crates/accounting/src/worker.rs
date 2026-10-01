// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's one writer: `RunWorker`, the state it holds, the commands
//! it carries out and the runs it drives (accounting-SPEC.md 8-11).
//!
//! It reaches this machine only through the [`hands::Hands`] it is built
//! with. The production value is made by the binary's assembly root
//! (`bin::assembly::production::hands`), which also starts the thread the
//! worker runs on; `fixture::hands` is the value its tests are built with.
//!
//! **This file holds the worker and the modules below hold its methods.**
//! `RunWorker` is declared here, so its private fields are
//! visible throughout `worker` and nowhere else — a private item
//! reaches the module that declares it and that module's descendants, so
//! the split cost no field its privacy. What stays here is what every
//! submodule needs and no submodule owns: the worker itself, the two
//! hooks a live control surface installs, and the door a `Command`
//! enters by. The lines it appends live in `recording`; opening and
//! closing in `lifetime`; the test fixtures in `fixture`. The writer's
//! loop is `attend`, commands wait on the `desk`, and runs are driven on
//! the lanes of the `pool` and write back through the `relay`.
//!
//! The `use` block below is where the submodules see each other. A
//! submodule imports from `super`, so what one part of the worker offers
//! another is stated once, here, and reads as a list rather than as a
//! graph; the one sibling reach is `credentials::dialect_headers`, which
//! the dispatching modules read where the credentials module keeps it.

pub mod attend;
mod booking;
pub mod chain_halt;
mod collaborating;
mod commanding;
mod credentials;
mod desk;
mod dispatching;
mod doorstep;
mod driving;
pub mod folds;
mod freezing;
pub mod genesis;
pub mod hands;
pub mod health;
mod keeping_warm;
mod lifetime;
mod mcp;
mod models;
mod naming;
pub mod opening_cost;
mod plans;
mod pool;
mod probing;
mod recording;
mod registering;
mod relay;
mod reviewing;
mod rooms;
mod settling;
mod toolkits;
mod waking;
mod workbench;

use collaborating::Collaborating;
use commanding::entrance::Entrance;
use credentials::held::Credentials;
use credentials::{Ceilings, Chosen, Credential, Entered, tuning_of};
use desk::Posted;
pub use desk::{CommandDesk, DeskWait};
use dispatching::Dispatched;
pub use dispatching::acp_dispatch;
use dispatching::running::Continuation;
use dispatching::{Agreed, Assignment, Given, Handover, Knock, run_id_for};
use doorstep::Doorstep;
use driving::flight::{Flight, Landed};
use driving::lane::{DriveContext, drive_run};
use driving::owing::{KnockChain, Owed, Owing, Unasked};
use driving::{Driven, Driving};
pub use folds::Standing;
pub use folds::start_served_views;
use folds::{Governance, INBOX_CAPACITY, SessionOrigins, new_inbox};
use genesis::city_segment;
pub use genesis::{Adopt, InitReport};
use hands::{Browsers, DesktopProgram, Hands};
pub use lifetime::Closing;
use lifetime::LedgerOpening;
use mcp::mounts_under;
use models::GatewayModels;
use naming::{building_of, governed_of, naming_edit_of, not_built, scope_of};
use plans::Reporter;
use plans::held::{PlanHolders, Planning};
pub use pool::Memory;
use recording::Stamping;
use rooms::{QueueTenure, RoomQueues};
use settling::{Ending, Settling, Sweep};
use workbench::{CITY_VERIFIER, Desks, Site, Workbench, held};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{AxError, EventRecord, RunId, TimeMs};
// What the test fixtures below reach through `super::*`, now that the
// lines this worker appends live in `recording`.
#[cfg(test)]
use crate::effect;
#[cfg(test)]
use kernel::{Address, AxCode, EventDraft, EventKind, Payload};
use runtime::Interrupt;
use storage::{Cas, JsonlLedger};

/// What the startup scan found and repaired.
pub struct ScanReport {
    /// What opening the ledger cut, told after the counts.
    pub opening: LedgerOpening,
    pub lines: usize,
    pub closed_calls: usize,
    /// The one count a caller branches on rather than prints: `resume`
    /// adds a line telling the person where to answer. `lines` and
    /// `closed_calls` reach nobody outside `summary`, so they stay in.
    pub waiting_approvals: usize,
}

impl ScanReport {
    /// What a person reads: what was verified, what was closed, what is
    /// still owed an answer, and, on a second line, what opening the
    /// ledger cut from a torn tail.
    #[must_use]
    pub fn summary(&self) -> String {
        let counts = format!(
            "{} line(s) verified; {} unknown-outcome call(s) closed; {} approval(s) waiting",
            self.lines, self.closed_calls, self.waiting_approvals
        );
        match self.opening.notice() {
            None => counts,
            Some(notice) => format!("{counts}\n{notice}"),
        }
    }
}

/// Where a served city listens, installed once by whoever serves it.
///
/// The four sinks are one fact — *somebody is watching this city* —
/// and a worker that has any of them has all of them. A worker driven
/// one command at a time has none, and that absence is the switch: its
/// runs ask their provider for no stream at all, so replay and citysim
/// take the byte-identical path they always took.
pub struct Serving {
    /// Where a model's text goes while it is still arriving.
    pub deltas: Arc<dyn Fn(wire::Delta) + Send + Sync>,
    /// Where a running command's output goes while it is still written.
    pub outputs: Arc<dyn Fn(wire::LiveOutput) + Send + Sync>,
    /// Where a fresh look at this machine goes: the one place the
    /// doctor's answer is replaced after the look taken at start-up.
    pub machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    /// What a running dispatch asks at its safe points.
    ///
    /// One handle per drive rather than one hook lent out and taken
    /// back: N runs may be asking at once, and each asks about itself
    /// (sprawling-SPEC.md 8-46-1).
    pub interrupts: Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
}

/// Runs the work a Command asks for. It owns the ledger, so the city has
/// one writer; commands reach it through a desk, and the socket task
/// that accepted them is free again immediately.
pub struct RunWorker {
    city_root: PathBuf,
    /// Which city this is, as the genesis line hashes. Read from the
    /// ledger the first time a commit needs signing and remembered, so
    /// no checkpoint, landing or merge re-reads the front of the history for
    /// it (sprawling-SPEC.md 8-51). Lazy rather than read on
    /// open, because a worker over a city with no genesis line yet is a
    /// legal state.
    city: std::sync::OnceLock<kernel::B3Hash>,
    ledger: JsonlLedger,
    /// What opening `ledger` repaired, kept until a person is told.
    opening: LedgerOpening,
    cas: Cas,
    /// The store every lane writes through: one handle, opened with the
    /// worker, because opening a store sweeps its half-written objects
    /// and a lane that opened its own would sweep another lane's put
    /// (sprawling-SPEC.md 8-113).
    lane_store: Arc<std::sync::Mutex<Cas>>,
    /// Whose identity this city can call which model under
    /// (`credentials::held`).
    credentials: Credentials,
    /// The four places a live control surface listens, or `None` in a
    /// worker driven one command at a time.
    ///
    /// **One `Option`, not four.** The four sinks are installed by
    /// one caller in one breath and are absent together in every other
    /// worker; as four fields the type admitted sixteen states of which
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
    log: recording::Notes,
    /// What reached the city's door and has not yet become a run
    /// (`doorstep`).
    doorstep: Doorstep,
    /// What each room's current session branched from, until the run
    /// that begins it is written (`crate::worker::folds::session`).
    pub(in crate::worker) origins: SessionOrigins,
    /// Every run in a lane right now, the crossing those lanes write
    /// history through, what the city owes each one when it comes home,
    /// the one checkpoint they take turns at, and the commands they left
    /// running. One per city, so the number of runs a city drives at once
    /// has one answer (sprawling-SPEC.md 8-46-2).
    flight: Flight,
    /// Where each line of the history sits, folded once and refreshed
    /// with what was appended since, so a question about one line reads
    /// that line rather than the whole history (sprawling-SPEC.md 8-82).
    pub(in crate::worker) index: storage::LedgerIndex,
    /// The keep-warm doors of runs that have landed, one per room
    /// (`keeping_warm`); empty under the default setting.
    warm: keeping_warm::Kept,
    /// Builds the adapter each run talks to (`models`). Received rather
    /// than built, so a second factory can drive a dispatch this worker
    /// accounts for.
    models: Box<dyn crate::ModelFactory + Send>,
    /// Connects the MCP servers a building's configuration names, and
    /// keeps them connected between runs (`mcp::Residents`). Received
    /// for the same reason `models` is. Shared, because the lane that
    /// prepares a dispatch connects its servers (sprawling-SPEC.md 8-113).
    connectors: Arc<dyn crate::Connectors + Send + Sync>,
    /// Looks at the machine this city runs on and installs onto it
    /// (`doctor::ThisMachine`). Received for the same reason `models`
    /// is.
    machine: Box<dyn crate::Machine + Send>,
    /// What time it is, for this worker and every lane it drives
    /// (`SystemClock`). Shared, because a lane reads it while the
    /// worker does.
    pub clock: std::sync::Arc<dyn crate::Clock + Send + Sync>,
    /// Reads the city's volume at the door new work enters by
    /// (sprawling-SPEC.md 8-116).
    read_volume: fn(&Path) -> Option<kernel::degradation::VolumeSpace>,
    /// Hands one of this city's paths to the desktop's file manager
    /// (`revealing::reveal`). Received rather than called, because it
    /// starts a program on the host (sprawling-SPEC.md 8-60).
    reveal: fn(&Path, &kernel::Address) -> Result<(), AxError>,
    /// Builds the browser tools a building's rules ask for
    /// (`browser_tool::for_rules`). Received rather than called, because
    /// a browser tool starts a browser on the host (sprawling-SPEC.md
    /// 8-45-2).
    browsers: Browsers,
    /// Where the desktop server a building's rules ask for is started
    /// from (`std::env::current_exe`). Received rather than asked,
    /// because it starts a program on the host (sprawling-SPEC.md 8-4d).
    desktop_program: DesktopProgram,
    /// Starts the official harness a room's resident names, in the room's
    /// tree (`HarnessProcess::start`). Received as a value the tests
    /// replace, because it starts a program on the host
    /// (sprawling-SPEC.md 8-124).
    harnesses: driving::harness::StartHarness,
    /// How this build installs one named item on this platform
    /// (`doctor::recipe_for`). Received rather than read, because the
    /// requirement table stays with the doctor (sprawling-SPEC.md,
    /// `doctor_install`).
    recipe_for: fn(&str) -> Result<&'static crate::Recipe, AxError>,
    /// Where the exec tool's interpreter, shell and engine come from
    /// (`bin::doctor::host`). Received rather than asked, because each
    /// reads this machine (accounting-SPEC.md 8-11).
    exec_host: hands::ExecHost,
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
        let read = storage::Provenance::city_of(
            &kernel::layout::CityLayout::new(&self.city_root).ledger(),
        )
        .map_err(storage::StorageError::into_ax)?;
        Ok(*self.city.get_or_init(|| read))
    }

    /// Sends every appended record to `sink` once it is durable.
    pub fn observe(&mut self, sink: Box<dyn FnMut(&EventRecord) + Send>) {
        self.ledger.observe(sink);
    }

    /// Takes the four places a served city listens.
    ///
    /// Separate from [`Self::observe`] because the two carry different
    /// kinds of thing: that one carries history, and these carry a view
    /// of work in progress and of the machine under it. One call rather
    /// than three, so a worker cannot end up streaming to a page that
    /// cannot interrupt it.
    ///
    /// The backlog takes the output sink here, so a worker nobody serves
    /// reads no command's output at all (runtime-SPEC 8-28-3).
    pub fn serve(&mut self, serving: Serving) {
        let outputs = Arc::clone(&serving.outputs);
        self.flight.backlog = self.flight.backlog.clone().with_sink(runtime::Sink::new(
            move |chunk: runtime::Chunk| {
                outputs(wire::LiveOutput {
                    run: chunk.run,
                    stream: match chunk.stream {
                        runtime::Stream::Out => wire::OutputStream::Out,
                        runtime::Stream::Err => wire::OutputStream::Err,
                    },
                    text: String::from_utf8_lossy(&chunk.bytes).into_owned(),
                });
            },
        ));
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
pub mod fixture;

/// What a person reads on a building's page after the commands that
/// shape the building: the page is `views::building_page`, the commands
/// are this module's, and the tests drive the commands.
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
mod building_page_tests;
