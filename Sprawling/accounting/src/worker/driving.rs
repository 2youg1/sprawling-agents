// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::RunId;
use kernel::{Address, AxError};
use runtime::bench::ToolBench;
use runtime::run::RunPlan;
use runtime::{SieveSite, package_exec};

use super::{RunWorker, Site};

mod entering;
pub mod flight;
pub(in crate::worker) mod harness;
pub mod lane;
pub(in crate::worker) mod owing;
mod placing;

pub(crate) struct Driving {
    pub adapter: super::keeping_warm::Door,

    pub bench: ToolBench,

    pub signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,

    pub(in crate::worker) policy: super::rooms::PolicySlot,

    pub write_root: PathBuf,

    pub checkpoint_scope: Vec<String>,

    pub checkpoint_entropy_allow: kernel::EntropyAllowlist,
    pub run_id: RunId,

    pub of: storage::Provenance,

    pub sieving: Sieving,

    pub member: Option<runtime::BacklogId>,

    pub delivered: std::collections::BTreeSet<Address>,

    pub plan: RunPlan,
    pub handoff: runtime::handoff::Handoff,

    pub(in crate::worker) workbench: super::Workbench,
}

pub(crate) struct Sieving {
    pub cas: std::sync::Arc<std::sync::Mutex<storage::Cas>>,
    pub city_root: PathBuf,
    pub room: Address,

    pub origin: storage::BlockOrigin,
    pub table: runtime::FilterTable,
    pub history: runtime::SieveHistory,

    pub stamps: runtime::StampGate,

    pub clock: runtime::ClockReading,
}

impl Sieving {
    fn package(
        &mut self,
        call: &kernel::ToolCall,
        outcome: kernel::ToolOutcome,
        temporal: kernel::Temporal,
    ) -> Result<kernel::ToolOutcome, AxError> {
        let stamp = match self.clock.latest() {
            Some(at) => self.stamps.observe(at, temporal)?,
            None => None,
        };
        let mut cas = super::held(&self.cas, "take the lanes' store")?;
        package_exec(
            call,
            outcome,
            SieveSite {
                offload: runtime::offload::OffloadSite {
                    cas: &mut cas,
                    city_root: &self.city_root,
                    room: &self.room,
                    origin: self.origin.clone(),
                },
                table: &self.table,
                history: &mut self.history,
            },
            stamp,
        )
    }
}

impl Sieving {
    pub(super) fn package_connector(
        &mut self,
        outcome: kernel::ToolOutcome,
    ) -> Result<kernel::ToolOutcome, AxError> {
        let mut cas = super::held(&self.cas, "take the lanes' store")?;
        runtime::package_connector(
            outcome,
            runtime::offload::OffloadSite {
                cas: &mut cas,
                city_root: &self.city_root,
                room: &self.room,
                origin: self.origin.clone(),
            },
        )
    }
}

pub struct Driven {
    pub outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,

    pub adapter: super::keeping_warm::Door,

    pub checkpointed: Vec<String>,

    pub ran: (u32, u32),

    pub raised: Vec<kernel::ApprovalItem>,

    pub(in crate::worker) workbench: super::Workbench,
}

impl Sieving {
    pub(in crate::worker) fn for_run(
        store: &std::sync::Arc<std::sync::Mutex<storage::Cas>>,
        site: &Site,
        addr: &Address,
    ) -> Sieving {
        Sieving {
            cas: std::sync::Arc::clone(store),
            city_root: site.write_root.clone(),
            room: addr.clone(),
            origin: storage::BlockOrigin {
                run: site.run_id,
                building: addr.clone(),
            },
            table: site.filters.clone(),
            history: runtime::SieveHistory::default(),
            stamps: runtime::StampGate::new(
                site.config.clock_stamp,
                site.config.clock_zones.clone(),
            ),
            clock: site.clock.clone(),
        }
    }
}

impl RunWorker {
    pub(in crate::worker) fn drive_context(&self) -> lane::DriveContext {
        lane::DriveContext {
            watching: self
                .serving
                .as_ref()
                .map(|at| std::sync::Arc::clone(&at.deltas)),
            person: self
                .serving
                .as_ref()
                .map(|at| std::sync::Arc::clone(&at.interrupts)),
            backlog: self.flight.backlog.clone(),
            clock: std::sync::Arc::clone(&self.clock),
            monotonic: self.monotonic,
        }
    }
}

#[cfg(test)]
mod tests;
