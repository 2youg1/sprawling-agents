// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The half of a dispatch's preparation that waits on the disk or on
//! another process: the review tree, the bench with its MCP servers,
//! and the frozen plan. It runs in the lane that drives the run, so the
//! accounting thread never waits on a handshake or a checkout
//! (`crates/sprawling/Spec.lean` §8-113). Once the run is driven and handed home,
//! the same lane puts the city's stock back, so the next placement is a
//! rename rather than a checkout and the run's landing does not wait on
//! the checkout (`crates/sprawling/Spec.lean` §8-145, §8-155, §8-161).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use kernel::{AxCode, AxError, Ledger, Locator, RunId};

use super::super::driving::Sieving;
use super::super::driving::harness::{HarnessDriven, HarnessHalf, drive_harness};
use super::super::recording::Notes;
use super::super::workbench::{BenchDesks, Laying, Placing, held};
use super::super::{Assignment, DriveContext, Driven, Driving, Given, Site, Stamping, drive_run};

/// One dispatch the accounting thread has decided, on its way to a
/// lane: what was asked, and everything the lane reads, owned so the
/// value can leave the thread. One arm per resident, decided once by
/// `agree_to_work` (`crates/sprawling/Spec.lean` §8-124).
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
    /// line the lane half writes (`crates/sprawling/Spec.lean` §8-41).
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

    /// Prepares the run in this lane, drives it, hands what it flew to
    /// `home`, and only then puts the city's stock back when the run
    /// borrowed a tree. The run's landing therefore never waits on the
    /// stock's checkout, and the pool's lane, the one caller outside
    /// tests, makes that checkout off the accounting thread
    /// (`crates/sprawling/Spec.lean` §8-145, §8-161).
    ///
    /// Generic in the ledger for the reason [`drive_run`] is: a lane
    /// writes through its relay, and a test on the accounting thread
    /// writes through the city's own ledger.
    pub(crate) fn fly<L: Ledger>(
        self,
        ledger: &mut L,
        context: DriveContext,
        home: impl FnOnce(Flown),
    ) {
        let (flown, restock) = match self {
            Staged::Model { at, mut site, lane } => {
                let restock = Restock {
                    city_root: lane.laying.city_root.clone(),
                    notes: lane.laying.notes.clone(),
                    staged_at: lane.laying.staged_at,
                };
                let driven = lane
                    .prepare(&at, &mut site, ledger, &context)
                    .and_then(|driving| drive_run(driving, ledger, context));
                let restock = restock.owed_by(site.lease.as_ref());
                (Flown::Model { at, site, driven }, restock)
            }
            Staged::Harness { at, half } => {
                let restock = Restock {
                    city_root: half.city_root.clone(),
                    notes: half.notes.clone(),
                    staged_at: half.staged_at,
                };
                let driven = drive_harness(half, ledger, context);
                let restock = restock.owed_by(driven.lease.as_ref());
                (Flown::Harness { at, driven }, restock)
            }
        };
        home(flown);
        if let Some(restock) = restock {
            restock.put_back();
        }
    }
}

/// What a lane needs once its run is driven to put the city's stock
/// back, and to say so when it cannot (`crates/sprawling/Spec.lean` §8-155).
struct Restock {
    city_root: PathBuf,
    notes: Notes,
    /// Where the ledger stood when the dispatch was staged, which the
    /// lane's diagnostic lines are anchored at (`crates/sprawling/Spec.lean` §8-113).
    staged_at: kernel::Seq,
}

impl Restock {
    /// The restock a run owes, or `None` for a run that borrowed no
    /// tree: it took no stock, and a stock it made would only hold a
    /// tree's worth of disk.
    fn owed_by(self, borrowed: Option<&storage::WorktreeLease>) -> Option<Restock> {
        borrowed.map(|_| self)
    }

    /// Puts the stock back, unless another lane of this process is
    /// stocking the same city, and writes one diagnostic line saying
    /// what that cost or why it failed. A failure leaves the run as it
    /// ended: the next placement checks its tree out whole.
    fn put_back(self) {
        let Some(_turn) = StockingTurn::take(&self.city_root) else {
            return;
        };
        let stocked = storage::Worktrees::open(&self.city_root).and_then(|trees| trees.stock());
        let (level, message) = match stocked {
            Ok(work) => (
                runtime::diagnostics::Level::Trace,
                format!(
                    "put the stock back: {} files created, {} rewritten, {} removed",
                    work.created, work.rewritten, work.removed
                ),
            ),
            Err(refused) => (
                runtime::diagnostics::Level::Refuse,
                format!(
                    "could not put the stock back, so the next placement checks its tree out \
                     whole: {refused}"
                ),
            ),
        };
        self.notes.write(
            level,
            self.staged_at,
            "accounting::worker::dispatching::preparing",
            &message,
        );
    }
}

/// The cities whose stock a lane of this process is putting back now.
///
/// One writer process per city, so this is who is stocking each city.
/// Two stockings that both found no stock would race, and the loser
/// would take back the winner's registration under a placement that is
/// taking it over (`crates/sprawling/Spec.lean` §8-155).
static STOCKING: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// One lane's turn at stocking one city, given up when it is dropped.
struct StockingTurn<'a> {
    city_root: &'a Path,
}

impl<'a> StockingTurn<'a> {
    /// Takes the city's turn, or answers `None` while another lane holds
    /// it: that lane's stock is the one the next placement takes, and
    /// waiting for its checkout would only keep this lane alive one
    /// checkout longer.
    fn take(city_root: &'a Path) -> Option<StockingTurn<'a>> {
        STOCKING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(city_root.to_path_buf())
            .then(|| StockingTurn { city_root })
    }
}

impl Drop for StockingTurn<'_> {
    /// A lock a dead lane left behind is taken all the same: every step
    /// under it is whole, and a turn never given back would stop this
    /// city's stock for the rest of the serving.
    fn drop(&mut self) {
        STOCKING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(self.city_root);
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

#[cfg(test)]
mod tests;
