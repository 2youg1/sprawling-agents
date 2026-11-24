// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use kernel::{AxCode, AxError, Ledger, Locator, Payload, RunId};

use super::super::driving::Sieving;
use super::super::driving::harness::{HarnessDriven, HarnessHalf, drive_harness};
use super::super::recording::{Notes, RunOpening};
use super::super::workbench::{BenchDesks, Laying, Placing, held};
use super::super::{Assignment, DriveContext, Driven, Driving, Given, Site, Stamping, drive_run};
use super::heavy_landing::{HeavyLanding, Swept};

#[expect(
    clippy::large_enum_variant,
    reason = "one value per run in flight, moved a handful of times on its way to a lane; a box buys nothing a run would notice"
)]
pub(in crate::worker) enum Staged {
    Model {
        at: Assignment,
        site: Site,
        lane: LaneHalf,
    },

    Harness {
        at: Assignment,
        half: HarnessHalf,
    },
}

pub(in crate::worker) struct LaneHalf {
    pub(in crate::worker) given: Given,
    pub(in crate::worker) job_locator: Locator,
    pub(in crate::worker) desks: BenchDesks,
    pub(in crate::worker) laying: Laying,

    pub(in crate::worker) inherited: super::super::freezing::InheritedContext,
    pub(in crate::worker) carried_from: Option<RunId>,
    pub(in crate::worker) member: Option<runtime::BacklogId>,

    pub(in crate::worker) command: Option<kernel::IdemKey>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "one value per run coming home, moved once from the lane to its landing; a box buys nothing a run would notice"
)]
pub(crate) enum Flown {
    Model {
        at: Assignment,
        site: Site,
        driven: Result<Driven, AxError>,

        swept: Option<Swept>,
    },
    Harness {
        at: Assignment,
        driven: HarnessDriven,
    },
}

impl Staged {
    pub(in crate::worker) fn assignment(&self) -> &Assignment {
        match self {
            Staged::Model { at, .. } | Staged::Harness { at, .. } => at,
        }
    }

    pub(crate) fn run_id(&self) -> RunId {
        match self {
            Staged::Model { site, .. } => site.run_id,
            Staged::Harness { half, .. } => half.chartered.run,
        }
    }

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
                let heavy = HeavyLanding {
                    city_root: lane.laying.city_root.clone(),
                    store: std::sync::Arc::clone(&lane.laying.store),
                    notes: lane.laying.notes.clone(),
                    staged_at: lane.laying.staged_at,
                };
                let mut ledger = RunOpening {
                    ledger,
                    command: lane.command,
                    session: at.resume,
                    image_input: site.model.input == kernel::InputKinds::TextImage,
                };
                let driven = lane
                    .prepare(&at, &mut site, &mut ledger, &context)
                    .and_then(|driving| drive_run(driving, &mut ledger, context));
                let swept = heavy.carry_out(&at, &site, &driven);
                let restock = restock.owed_by(site.lease.as_ref());
                (
                    Flown::Model {
                        at,
                        site,
                        driven,
                        swept,
                    },
                    restock,
                )
            }
            Staged::Harness { at, half } => {
                let restock = Restock {
                    city_root: half.city_root.clone(),
                    notes: half.notes.clone(),
                    staged_at: half.staged_at,
                };
                let mut ledger = RunOpening {
                    ledger,
                    command: half.command,
                    session: at.resume,
                    image_input: false,
                };
                let driven = drive_harness(half, &mut ledger, context);
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

struct Restock {
    city_root: PathBuf,
    notes: Notes,

    staged_at: kernel::Seq,
}

impl Restock {
    fn owed_by(self, borrowed: Option<&storage::WorktreeLease>) -> Option<Restock> {
        borrowed.map(|lease| Restock {
            city_root: lease.owner().to_owned(),
            ..self
        })
    }

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

static STOCKING: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

struct StockingTurn<'a> {
    city_root: &'a Path,
}

impl<'a> StockingTurn<'a> {
    fn take(city_root: &'a Path) -> Option<StockingTurn<'a>> {
        STOCKING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(city_root.to_path_buf())
            .then(|| StockingTurn { city_root })
    }
}

impl Drop for StockingTurn<'_> {
    fn drop(&mut self) {
        STOCKING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(self.city_root);
    }
}

impl LaneHalf {
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
        for next in &workbench.drift {
            laying.drift_book.record(next, || {
                Stamping {
                    ledger: &mut *ledger,
                    command,
                    clock,
                }
                .record_for(
                    site.run_id,
                    crate::effect::Line {
                        who: "city".to_owned(),
                        addr: next.building.clone(),
                        kind: kernel::EventKind::DigestInvalidated,
                        data: Payload::of(&next.drift)?,
                    },
                )
            })?;
        }
        let (plan, handoff) = {
            let mut store = held(&laying.store, "take the lanes' store")?;
            super::super::freezing::Freezing {
                city_root: &laying.city_root,
                cas: &mut store,
                inherited: inherited.messages,
                carried_from,
            }
            .freeze_plan(site, &mut workbench, at, given)?
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

        let Some(adapter) = site.adapter.take() else {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "carry the adapter into the drive",
                "the site arrived without one",
            )
            .with_recovery("report this: stand_up always seats an adapter"));
        };
        Ok(Driving {
            city_root: laying.city_root.clone(),
            adapter,
            bench,
            signals: std::sync::Arc::clone(&desks.signals),
            policy: desks.policy.clone(),
            write_root: site.write_root.clone(),
            checkpoint_scope,
            checkpoint_entropy_allow: site.rules.checkpoint_entropy_allow().clone(),
            run_id: site.run_id,
            of: site.provenance(laying.city, &at.addr),
            sieving,
            member,
            plan,
            handoff,
            delivered: inherited.delivered,
            workbench,
        })
    }
}

#[cfg(test)]
mod tests;
