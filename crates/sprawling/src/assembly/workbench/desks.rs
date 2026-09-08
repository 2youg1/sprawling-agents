// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The desks one dispatch lends out: opened together, lent what they
//! hold, and taken back together when the drive ends.

use kernel::{Address, AxError};

use super::super::{RunWorker, new_inbox, now_ms};
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
    /// # Errors
    /// Propagates a plan that cannot be read and a shelf that cannot be
    /// indexed - both before a model is called, since a run built on
    /// either would spend a call to produce claims the city was always
    /// going to drop.
    pub(in crate::assembly) fn open_desks(
        &mut self,
        site: &Site,
        addr: &Address,
    ) -> Result<Desks, AxError> {
        let pr = std::rc::Rc::new(std::cell::RefCell::new(collab::PrDesk::new(
            site.who.clone(),
            addr.clone(),
            site.branch.clone(),
            site.branch
                .as_deref()
                .and_then(|name| collab::NodeId::parse(name).ok()),
            self.requests.clone(),
        )));

        // The room's queue is lent to the desk for the length of the
        // drive and taken back below. One queue exists per room at all
        // times; a copy would be a second answer to "what arrived first".
        let lent = self.inboxes.remove(addr).unwrap_or_else(new_inbox);
        let waiting = lent.pending();
        let signals = std::rc::Rc::new(std::cell::RefCell::new(collab::SignalDesk::new(
            site.run_id,
            addr.clone(),
            site.who.clone(),
            site.building.addr().clone(),
            now_ms()?,
            lent,
        )));
        let goals = std::rc::Rc::new(std::cell::RefCell::new(collab::GoalDesk::new(
            site.run_id,
            site.who.clone(),
            self.goals.clone(),
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
        let plan = std::rc::Rc::new(std::cell::RefCell::new(collab::ClaimDesk::new(
            site.who.clone(),
            addr.clone(),
            plan_text,
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
        let shelf = std::rc::Rc::new(std::cell::RefCell::new(collab::ArchiveDesk::new(
            addr.clone(),
            held,
        )));
        Ok(Desks {
            signals,
            goals,
            plan,
            shelf,
            pr,
            plan_path,
            waiting,
        })
    }
}
