// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
mod errand;
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
pub use reviewing::{PublicationTarget, publish_project};
mod rooms;
mod settling;
mod toolkits;
mod waking;
mod workbench;

pub(crate) fn catalog_builtins(
    search: Option<&kernel::config::SearchSupplier>,
) -> Result<Vec<(&'static str, kernel::ToolDef)>, kernel::AxError> {
    workbench::catalog_builtins(search)
}

use collaborating::Collaborating;
use commanding::entrance::Entrance;
use credentials::held::Credentials;
use credentials::{Chosen, Credential, Entered, Stated};
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
pub use genesis::{Adopt, InitReport};
use genesis::{city_segment, prompt_segment};
use hands::{Browsers, DesktopProgram, Hands};
use lifetime::LedgerOpening;
pub use lifetime::{ClosedBy, Closing};
use models::GatewayModels;
use naming::{building_of, governed_of, naming_edit_of, not_built, scope_of};
use plans::Reporter;
use plans::held::{PlanHolders, Planning};
pub use pool::Memory;
use recording::Stamping;
pub use relay::Relay;
use rooms::{QueueTenure, RoomQueues};
use settling::{Ending, Settling, Sweep};
use workbench::{CITY_VERIFIER, Desks, DriftBook, Site, Workbench, held};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{AxError, EventRecord, RunId, TimeMs};

#[cfg(test)]
use crate::effect;
#[cfg(test)]
use kernel::{Address, AxCode, EventDraft, EventKind, Payload};
use runtime::Interrupt;
use storage::{Cas, JsonlLedger};

pub struct ScanReport {
    pub opening: LedgerOpening,
    pub lines: usize,
    pub closed_calls: usize,

    pub frozen_runs: usize,

    pub waiting_approvals: usize,
}

impl ScanReport {
    #[must_use]
    pub fn summary(&self) -> String {
        let counts = format!(
            "{} line(s) verified; {} unknown-outcome call(s) closed; {} lost run(s) frozen; \
             {} approval(s) waiting",
            self.lines, self.closed_calls, self.frozen_runs, self.waiting_approvals
        );
        match self.opening.notice() {
            None => counts,
            Some(notice) => format!("{counts}\n{notice}"),
        }
    }
}

pub struct Serving {
    pub deltas: Arc<dyn Fn(wire::Delta) + Send + Sync>,

    pub outputs: Arc<dyn Fn(wire::LiveOutput) + Send + Sync>,

    pub machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,

    pub interrupts: Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
}

pub struct RunWorker {
    city_root: PathBuf,

    city: std::sync::OnceLock<kernel::B3Hash>,
    ledger: JsonlLedger,
    recovered_projects: std::collections::BTreeSet<kernel::Address>,

    opening: LedgerOpening,
    cas: Cas,

    lane_store: Arc<std::sync::Mutex<Cas>>,

    credentials: Credentials,

    serving: Option<Serving>,

    governance: Governance,

    collaborating: Collaborating,

    planning: Planning,

    last_tick: TimeMs,

    log: recording::Notes,

    doorstep: Doorstep,

    pub(in crate::worker) origins: SessionOrigins,

    flight: Flight,

    pub(in crate::worker) index: storage::LedgerIndex,

    warm: keeping_warm::Kept,

    models: Box<dyn crate::ModelFactory + Send>,

    connectors: Arc<dyn crate::Connectors + Send + Sync>,

    drift_book: DriftBook,

    machine: Box<dyn crate::Machine + Send>,

    pub clock: std::sync::Arc<dyn crate::Clock + Send + Sync>,

    monotonic: fn() -> std::time::Instant,

    read_volume: fn(&Path) -> Option<kernel::degradation::VolumeSpace>,

    reveal: fn(&Path, &kernel::Address) -> Result<(), AxError>,

    browsers: Browsers,

    desktop_program: DesktopProgram,

    harnesses: driving::harness::StartHarness,

    recipe_for: fn(&str) -> Result<&'static crate::Recipe, AxError>,

    exec_host: hands::ExecHost,
}

impl RunWorker {
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

    pub fn observe(&mut self, sink: Box<dyn FnMut(&EventRecord) + Send>) {
        self.ledger.observe(sink);
    }

    pub fn attachment_store(&self) -> Arc<std::sync::Mutex<Cas>> {
        Arc::clone(&self.lane_store)
    }

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod the_scan_summary {
    use super::*;

    #[test]
    fn joins_its_counts_with_one_space() {
        let report = ScanReport {
            opening: LedgerOpening::Intact,
            lines: 3,
            closed_calls: 1,
            frozen_runs: 2,
            waiting_approvals: 4,
        };
        assert_eq!(
            report.summary(),
            "3 line(s) verified; 1 unknown-outcome call(s) closed; 2 lost run(s) frozen; 4 approval(s) waiting"
        );
    }
}
