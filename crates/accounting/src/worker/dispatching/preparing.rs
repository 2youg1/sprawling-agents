// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The half of a dispatch's preparation that waits on the disk or on
//! another process: the review tree, the bench with its MCP servers,
//! and the frozen plan. It runs in the lane that drives the run, so the
//! accounting thread never waits on a handshake or a checkout
//! (sprawling-SPEC.md 8-113).

use kernel::{AxCode, AxError, Ledger, Locator, RunId};

use super::super::driving::Sieving;
use super::super::driving::harness::{HarnessDriven, HarnessHalf, drive_harness};
use super::super::workbench::{BenchDesks, Laying, Placing, held};
use super::super::{Assignment, DriveContext, Driven, Driving, Given, Site, Stamping, drive_run};

/// One dispatch the accounting thread has decided, on its way to a
/// lane: what was asked, and everything the lane reads, owned so the
/// value can leave the thread. One arm per resident, decided once by
/// `agree_to_work` (sprawling-SPEC.md 8-124).
#[expect(
    clippy::large_enum_variant,
    reason = "one value per run in flight, moved a handful of times on its way to a lane; a box buys nothing a run would notice"
)]
pub(in crate::worker) enum Staged {
    /// A model run: where it stands, and the half the lane prepares.
    Model {
        at: Assignment,
        site: Site,
        lane: LaneHalf,
    },
    /// A harness run: the half the lane drives.
    Harness { at: Assignment, half: HarnessHalf },
}

/// What only the lane half reads.
pub(in crate::worker) struct LaneHalf {
    pub(in crate::worker) given: Given,
    pub(in crate::worker) job_locator: Locator,
    pub(in crate::worker) desks: BenchDesks,
    pub(in crate::worker) laying: Laying,
    /// The conversation this run opens with, rebuilt on the accounting
    /// thread where the ledger's index is held.
    pub(in crate::worker) inherited: Vec<kernel::ChatMessage>,
    pub(in crate::worker) carried_from: Option<RunId>,
    pub(in crate::worker) member: Option<runtime::BacklogId>,
    /// The key of the command this dispatch answers, stamped on every
    /// line the lane half writes (sprawling-SPEC.md 8-41).
    pub(in crate::worker) command: Option<kernel::IdemKey>,
}

/// What a lane carries home, whether its preparation held or not: the
/// assignment and what the run borrowed go back to `land`, which gives
/// back the tree the run may hold before it reads how the drive went.
#[expect(
    clippy::large_enum_variant,
    reason = "one value per run coming home, moved once from the lane to its landing; a box buys nothing a run would notice"
)]
pub(crate) enum Flown {
    Model {
        at: Assignment,
        site: Site,
        driven: Result<Driven, AxError>,
    },
    Harness {
        at: Assignment,
        driven: HarnessDriven,
    },
}

impl Staged {
    pub(crate) fn run_id(&self) -> RunId {
        match self {
            Staged::Model { site, .. } => site.run_id,
            Staged::Harness { half, .. } => half.chartered.run,
        }
    }

    /// Prepares the run in this lane and drives it.
    ///
    /// Generic in the ledger for the reason [`drive_run`] is: a lane
    /// writes through its relay, and a test on the accounting thread
    /// writes through the city's own ledger.
    pub(crate) fn fly<L: Ledger>(self, ledger: &mut L, context: DriveContext) -> Flown {
        match self {
            Staged::Model { at, mut site, lane } => {
                let driven = lane
                    .prepare(&at, &mut site, ledger, &context)
                    .and_then(|driving| drive_run(driving, ledger, context));
                Flown::Model { at, site, driven }
            }
            Staged::Harness { at, half } => Flown::Harness {
                at,
                driven: drive_harness(half, ledger, context),
            },
        }
    }
}

impl LaneHalf {
    /// Places the tree, lays out the bench, freezes the plan and asks
    /// the successor's probe: everything between the accounting
    /// thread's decision and the first model call.
    ///
    /// # Errors
    /// Propagates the failures of each phase. The site keeps whatever
    /// it was lent before the failure, so the landing gives it back.
    fn prepare<L: Ledger>(
        self,
        at: &Assignment,
        site: &mut Site,
        ledger: &mut L,
        context: &DriveContext,
    ) -> Result<Driving, AxError> {
        let LaneHalf {
            given,
            job_locator,
            desks,
            laying,
            inherited,
            carried_from,
            member,
            command,
        } = self;
        let clock = &*context.clock;
        site.place_tree(
            at,
            &Placing {
                city_root: &laying.city_root,
                city: laying.city,
                clock,
                checkpoint_gate: &laying.checkpoint_gate,
            },
            &mut Stamping {
                ledger,
                command,
                clock,
            },
        )?;
        let mut workbench = laying.lay_out_workbench(site, &desks, at, &job_locator)?;
        let (plan, handoff) = {
            let mut store = held(&laying.store, "take the lanes' store")?;
            super::super::freezing::Freezing {
                city_root: &laying.city_root,
                cas: &mut store,
                inherited,
                carried_from,
            }
            .freeze_plan(site, &workbench, at, given)?
        };
        if let Some(handed) = at.succession.as_ref() {
            site.probe_after(
                &plan,
                handed,
                &mut Stamping {
                    ledger,
                    command,
                    clock,
                },
            )?;
        }
        let checkpoint_scope = site.checkpoint_scope()?;
        let sieving = Sieving::for_run(&laying.store, site, &at.addr);
        let bench = workbench.take_bench()?;
        // The adapter moves into the drive and comes home in `Driven`,
        // and it is taken last, so a failure above leaves it on the site.
        let Some(adapter) = site.adapter.take() else {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "carry the adapter into the drive",
                "the site arrived without one",
            )
            .with_recovery("report this: stand_up always seats an adapter"));
        };
        Ok(Driving {
            adapter,
            bench,
            signals: std::sync::Arc::clone(&desks.signals),
            write_root: site.write_root.clone(),
            checkpoint_scope,
            run_id: site.run_id,
            of: site.provenance(laying.city, &at.addr),
            sieving,
            member,
            plan,
            handoff,
            workbench,
        })
    }
}
