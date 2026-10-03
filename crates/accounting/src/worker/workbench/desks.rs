// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The desks one dispatch lends out: opened together, lent what they
//! hold, and taken back together when the drive ends.

use kernel::{Address, AxError};

use super::super::RunWorker;
use super::{Desks, Site};

impl RunWorker {
    /// Opens the desks this run works at, and lends them what they hold.
    ///
    /// Every desk is a place a tool writes to and the settlement reads
    /// back, so they open together and come back as one value. The
    /// room's queue moves into the signal desk rather than being copied
    /// there: one queue per room at all times, and a copy would be a
    /// second answer to what arrived first.
    ///
    /// A second run in the same room is given a queue of its own and
    /// told so in [`Desks::holding`]: the room table lends its queue to
    /// one reader, and what arrives meanwhile waits there for that
    /// reader to land (`crates/sprawling/Spec.lean` §8-46-9).
    ///
    /// # Errors
    /// Propagates a plan that cannot be read and a shelf that cannot be
    /// indexed - both before a model is called, since a run built on
    /// either would spend a call to produce claims the city was always
    /// going to drop.
    pub(in crate::worker) fn open_desks(
        &mut self,
        site: &Site,
        addr: &Address,
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

        // The room's queue is lent to the desk for the length of the
        // drive and given back when the run lands. The table keeps the
        // loan rather than the queue being lifted out of it, so a
        // second run in the same room is answered instead of being
        // handed a queue that would overwrite the first one's.
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

        // The plan is shared ground, so it is read from and written back
        // to the city even when the run writes everywhere else in its
        // own tree. A claim nobody else can see is not a claim; the work
        // stays private until it is checked, the fact that somebody is
        // doing it does not.
        let plan_path = city::roadmap_path(&self.city_root, site.building.addr());
        // A plan that is not there yet reads as empty; every other reason
        // this file cannot be read is reported here, before a model is
        // called. Reading them as empty spent a call to produce claims
        // that the compare-and-swap below was always going to drop, and
        // told the person a neighbour had moved their row.
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
                },
            ),
        )));

        // What the building already knows, computed from the shelf
        // rather than kept beside it. An index that was stored would be
        // a second copy of what the files say, and the files are the
        // ones that are true.
        // `archive_index` already answers `Ok(empty)` for a building with
        // no shelf, so anything it reports is a real failure and telling
        // the model this building knows nothing would be a lie about it.
        let held: Vec<collab::Held> = city::archive_index(&self.city_root, site.building.addr())?
            .into_iter()
            .map(|entry| collab::Held {
                kind: entry.kind.as_str().to_owned(),
                text: entry.subject,
            })
            .collect();
        let shelf = std::sync::Arc::new(std::sync::Mutex::new(collab::ArchiveDesk::new(
            addr.clone(),
            held,
        )));
        // Copied rather than lent: the join and the graph stay with the
        // worker, which answers a handback while this run is still going.
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
            pr,
            workshop,
            plan_path,
            waiting,
            tenure: lent.tenure,
        })
    }

    /// The post a run's signal desk writes through: each line crosses
    /// the relay at the call, so it is on the ledger while the run is
    /// still going, and the accounting thread delivers what it sent
    /// when it shows the line (collab D7).
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
                // Delivered on the accounting thread, which shows the
                // relayed line to the room table (`RunWorker::show_relayed`).
                crate::effect::Then::Deliver(_) | crate::effect::Then::Nothing => Ok(()),
                crate::effect::Then::Roadmap { .. } | crate::effect::Then::Shelf(_) => Ok(()),
            }
        })
    }

    /// The goal desk a run registers its ground through: each entry is
    /// decided on the accounting thread at the call, and its line is
    /// filed under `room`.
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
