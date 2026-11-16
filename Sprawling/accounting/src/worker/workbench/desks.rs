// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::{Address, AxError};

use super::super::RunWorker;
use super::Site;

impl RunWorker {
    pub(in crate::worker) fn open_desks(
        &mut self,
        site: &Site,
        addr: &Address,
        depth: kernel::Depth,
    ) -> Result<Desks, AxError> {
        let pr = std::sync::Arc::new(std::sync::Mutex::new(collab::PrDesk::new(
            site.who.clone(),
            addr.clone(),
            site.branch.clone(),
            site.branch
                .as_deref()
                .and_then(|name| collab::NodeId::parse(name).ok()),
            self.collaborating.requests.clone(),
        )));

        let lent = self.collaborating.rooms.lend(addr, site.run_id);
        let waiting = lent.inbox.pending();
        let signals = std::sync::Arc::new(std::sync::Mutex::new(collab::SignalDesk::new(
            site.run_id,
            addr.clone(),
            site.who.clone(),
            site.building.addr().clone(),
            self.clock.now()?,
            collab::RoomMail {
                inbox: lent.inbox,
                slot: lent.slot,
                post: self.post(site.run_id, addr, &site.who),
            },
        )));
        let goals = std::sync::Arc::new(std::sync::Mutex::new(self.goal_desk(
            site.run_id,
            addr,
            &site.who,
        )));

        let plan_path = city::roadmap_path(&self.city_root, site.building.addr());

        let plan_text = city::roadmap(&self.city_root, site.building.addr())?;
        let plan = std::sync::Arc::new(std::sync::Mutex::new(collab::ClaimDesk::new(
            site.who.clone(),
            addr.clone(),
            plan_text,
            crate::worker::booking::booking(
                self.bell(),
                crate::worker::booking::Claimant {
                    building: site.building.addr().clone(),
                    room: addr.clone(),
                    run: site.run_id,
                    who: site.who.clone(),
                    clock: std::sync::Arc::clone(&self.clock),
                    read: self.flight.gate.booked.read_mark(),
                },
            ),
        )));

        let held: Vec<collab::Held> = city::archive_index(&self.city_root, site.building.addr())?
            .into_iter()
            .map(|entry| collab::Held {
                kind: entry.kind.as_str().to_owned(),
                text: entry.subject,
            })
            .collect();
        let shelf_root = std::sync::Arc::new(std::sync::OnceLock::new());
        let shelf = std::sync::Arc::new(std::sync::Mutex::new(collab::ArchiveDesk::new(
            addr.clone(),
            held,
            self.filer(site, addr, std::sync::Arc::clone(&shelf_root)),
        )));

        let mut joined = collab::FanIn::new();
        if let Some(existing) = self.collaborating.joins.get(addr) {
            for artifact in existing.artifacts() {
                joined.accept(artifact.clone());
            }
        }
        let workshop = std::sync::Arc::new(std::sync::Mutex::new(collab::WorkshopDesk::new(
            site.who.clone(),
            joined,
            self.collaborating
                .workshops
                .get(addr)
                .map_or_else(std::collections::BTreeSet::new, |underway| {
                    underway.handed().clone()
                }),
        )));
        Ok(Desks {
            signals,
            goals,
            plan,
            shelf,
            shelf_root,
            pr,
            workshop,

            delegates: std::sync::Arc::new(std::sync::Mutex::new(collab::DelegateDesk::new(
                depth,
                site.building.addr().clone(),
                crate::worker::dispatching::agreeing::start_check(self.city_root.clone()),
            ))),
            plan_path,
            waiting,
            tenure: lent.tenure,
            policy: lent.policy,
        })
    }

    fn post(&self, run: kernel::RunId, room: &Address, who: &str) -> collab::Post {
        let mut relay = self.flight.gate.issue();
        let clock = std::sync::Arc::clone(&self.clock);
        let (room, who) = (room.clone(), who.to_owned());
        collab::Post::new(move |line: &collab::SignalEffect| {
            let landing = crate::effect::Landing::signals(vec![line.clone()], &room, &who)?;
            let mut stamping = crate::worker::recording::Stamping {
                ledger: &mut relay,
                command: None,
                clock: clock.as_ref(),
            };
            match landing.record(&mut |line| stamping.record_for(run, line))? {
                crate::effect::Then::Deliver(_) | crate::effect::Then::Nothing => Ok(()),
                crate::effect::Then::Roadmap { .. } | crate::effect::Then::Shelf(_) => Ok(()),
            }
        })
    }

    fn filer(
        &self,
        site: &Site,
        room: &Address,
        shelf_root: std::sync::Arc<std::sync::OnceLock<std::path::PathBuf>>,
    ) -> collab::Filer {
        let mut relay = self.flight.gate.issue();
        let clock = std::sync::Arc::clone(&self.clock);
        let (run, building) = (site.run_id, site.building.addr().clone());
        let (room, who) = (room.clone(), site.who.clone());
        collab::Filer::new(move |record: &collab::ArchiveEffect| {
            let write_root = shelf_root.get().ok_or_else(|| {
                AxError::failure(
                    kernel::AxCode::StorageFatal,
                    "archive something",
                    "the run's tree was not placed before the archive desk was called",
                )
                .with_recovery("report this against accounting::worker::dispatching: the lane places the tree before it lays out the bench")
            })?;
            let landing = crate::effect::Landing::shelf(
                vec![record.clone()],
                write_root,
                &building,
                clock.now()?,
                &room,
                &who,
            )?;
            let mut stamping = crate::worker::recording::Stamping {
                ledger: &mut relay,
                command: None,
                clock: clock.as_ref(),
            };
            let filings = match landing.record(&mut |line| stamping.record_for(run, line))? {
                crate::effect::Then::Shelf(filings) => filings,
                crate::effect::Then::Deliver(_)
                | crate::effect::Then::Nothing
                | crate::effect::Then::Roadmap { .. } => Vec::new(),
            };
            let mut filed = None;
            for filing in filings {
                city::file_archive(&filing.entry, &filing.body)?;
                filed = Some(collab::Held {
                    kind: filing.entry.kind.as_str().to_owned(),
                    text: filing.entry.subject,
                });
            }
            filed.ok_or_else(|| {
                AxError::failure(
                    kernel::AxCode::StorageFatal,
                    "archive something",
                    "the record was not turned into a shelf entry",
                )
                .with_recovery("report this against accounting::effect: a shelf landing files one entry per record")
            })
        })
    }

    pub(in crate::worker) fn goal_desk(
        &self,
        run: kernel::RunId,
        room: &Address,
        who: &str,
    ) -> collab::GoalDesk {
        collab::GoalDesk::new(
            run,
            who.to_owned(),
            super::super::registering::booking(self.bell(), run, room.clone()),
        )
    }
}

impl BenchDesks {
    pub(in crate::worker) fn archive_tool(
        &self,
        tree: &std::path::Path,
    ) -> Result<collab::ArchiveTool, AxError> {
        if let Err(placed) = self.shelf_root.set(tree.to_path_buf())
            && self.shelf_root.get() != Some(&placed)
        {
            return Err(AxError::failure(
                kernel::AxCode::StorageFatal,
                "lay out the archive desk",
                "the shelf was already pointed at another tree",
            )
            .with_recovery(
                "report this against accounting::worker::workbench: one run files in one tree",
            ));
        }
        collab::ArchiveTool::new(std::sync::Arc::clone(&self.shelf))
    }
}

pub(in crate::worker) struct Desks {
    pub(in crate::worker) signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    pub(in crate::worker) goals: std::sync::Arc<std::sync::Mutex<collab::GoalDesk>>,
    pub(in crate::worker) plan: std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>,
    pub(in crate::worker) shelf: std::sync::Arc<std::sync::Mutex<collab::ArchiveDesk>>,

    pub(in crate::worker) shelf_root: std::sync::Arc<std::sync::OnceLock<PathBuf>>,
    pub(in crate::worker) pr: std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,

    pub(in crate::worker) workshop: std::sync::Arc<std::sync::Mutex<collab::WorkshopDesk>>,
    pub(in crate::worker) delegates: std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,

    pub(in crate::worker) plan_path: PathBuf,

    pub(in crate::worker) waiting: u32,

    pub(in crate::worker) tenure: crate::worker::QueueTenure,

    pub(in crate::worker) policy: crate::worker::rooms::PolicySlot,
}

pub(in crate::worker) struct BenchDesks {
    pub(in crate::worker) signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    pub(in crate::worker) goals: std::sync::Arc<std::sync::Mutex<collab::GoalDesk>>,
    pub(in crate::worker) plan: std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>,
    pub(in crate::worker) shelf: std::sync::Arc<std::sync::Mutex<collab::ArchiveDesk>>,
    pub(in crate::worker) shelf_root: std::sync::Arc<std::sync::OnceLock<PathBuf>>,
    pub(in crate::worker) pr: std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,
    pub(in crate::worker) workshop: std::sync::Arc<std::sync::Mutex<collab::WorkshopDesk>>,
    pub(in crate::worker) delegates: std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
    pub(in crate::worker) waiting: u32,
    pub(in crate::worker) policy: crate::worker::rooms::PolicySlot,
}

impl Desks {
    pub(in crate::worker) fn for_bench(&self) -> BenchDesks {
        BenchDesks {
            signals: std::sync::Arc::clone(&self.signals),
            goals: std::sync::Arc::clone(&self.goals),
            plan: std::sync::Arc::clone(&self.plan),
            shelf: std::sync::Arc::clone(&self.shelf),
            shelf_root: std::sync::Arc::clone(&self.shelf_root),
            pr: std::sync::Arc::clone(&self.pr),
            workshop: std::sync::Arc::clone(&self.workshop),
            delegates: std::sync::Arc::clone(&self.delegates),
            waiting: self.waiting,
            policy: self.policy.clone(),
        }
    }
}
