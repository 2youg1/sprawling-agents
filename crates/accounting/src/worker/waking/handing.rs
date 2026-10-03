// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Work a driving run hands down, started at the call (collab D7), and
//! the graph it lays out, registered with its room at the call and
//! closed when the run is cancelled or fails (collab D14).
//!
//! The model is `crates/collab/spec/Workshop.lean`: a graph is laid out
//! (`layOut`), its nodes land in any order against their parent
//! (`land`), and the parent's ending closes it or leaves it open
//! (`parentEnds`, decided once, by [`GraphAfter::of`]). Order is ledger
//! seq and the injected clock, the same on Windows, macOS and Linux.

use std::sync::{Arc, Mutex};

use kernel::{Address, AxError, RunId};

use super::super::{Assignment, Owing, RunWorker, held};

const DELEGATE_DESK: &str = "read the delegate desk";
const WORKSHOP_DESK: &str = "read the workshop desk";

/// What the city keeps for one driving run so that what it hands down
/// starts while it still drives: the two desks its tools write to, and
/// how a child of it is dispatched.
pub(in crate::worker) struct Handing {
    room: Address,
    delegates: Arc<Mutex<collab::DelegateDesk>>,
    workshop: Arc<Mutex<collab::WorkshopDesk>>,
    policy: kernel::RunPolicy,
    taint: kernel::TaintSet,
    /// What every child of this run is owed: a handback to `room`.
    owing: Owing,
    graph: Graph,
}

/// Whether this run registered a graph with its room, which is the
/// graph its ending may close.
#[derive(Clone, Copy)]
enum Graph {
    NotLaidOut,
    Registered,
}

/// What a parent's ending does to the graph it laid out (collab D14,
/// `holds` in `spec/Workshop.lean`).
#[derive(Clone, Copy)]
pub(in crate::worker) enum GraphAfter {
    /// Done or Limit: nodes that become ready are still handed down.
    Open,
    /// Cancelled or failed: nothing more is handed down; nodes in
    /// flight run to the end and hand back.
    Closed,
}

impl GraphAfter {
    /// The one reading of D14: a parent that did not land Done or Limit
    /// hands nothing more down.
    pub(in crate::worker) fn of(
        driven: &Result<runtime::Run<runtime::run::Frozen>, AxError>,
    ) -> GraphAfter {
        match driven.as_ref().map(|run| run.completion()) {
            Ok(kernel::Completion::Done(_) | kernel::Completion::Limit) => GraphAfter::Open,
            Ok(kernel::Completion::Cancelled) | Err(_) => GraphAfter::Closed,
        }
    }
}

impl Handing {
    /// The hand-over for a run about to enter a lane, owing its children
    /// a handback to `at.addr`.
    pub(in crate::worker) fn new(
        at: &Assignment,
        delegates: Arc<Mutex<collab::DelegateDesk>>,
        workshop: Arc<Mutex<collab::WorkshopDesk>>,
        owing: &Owing,
    ) -> Handing {
        Handing {
            room: at.addr.clone(),
            delegates,
            workshop,
            policy: at.policy,
            taint: at.taint.clone(),
            owing: owing.child(at.addr.clone()),
            graph: Graph::NotLaidOut,
        }
    }
}

impl RunWorker {
    /// Registers what `run` laid out since the last look and starts every
    /// child it asked for since then. Called when the city shows the
    /// lines a lane wrote, so a call's effect follows the call rather than
    /// the run's landing.
    ///
    /// A child that cannot be started is noted rather than returned: the
    /// call already answered the model, and failing the relay over it
    /// would punish every other lane's lines.
    ///
    /// # Errors
    /// A desk left locked by a thread that died.
    pub(in crate::worker) fn hand_over_at_call(&mut self, run: RunId) -> Result<(), AxError> {
        let Some(handing) = self.collaborating.handing.get_mut(&run) else {
            return Ok(());
        };
        if let Some(underway) = held(&handing.workshop, WORKSHOP_DESK)?.take_underway() {
            handing.graph = Graph::Registered;
            self.collaborating
                .workshops
                .insert(handing.room.clone(), underway);
        }
        let fresh = held(&handing.delegates, DELEGATE_DESK)?.hand_over();
        let (room, policy, taint) = (handing.room.clone(), handing.policy, handing.taint.clone());
        let owing = handing.owing.child(room.clone());
        for work in fresh {
            let child = work.room.clone();
            self.note(
                runtime::diagnostics::Level::Effect,
                "collab::delegate",
                &format!("{} handed work to {}", room.as_str(), child.as_str()),
            );
            // Carried rather than defaulted: work handed down is the same
            // piece of work, done under the same policy and taint.
            let started = kernel::event::Who::resident(room.clone()).and_then(|by| {
                self.dispatch_into_lane(
                    Assignment {
                        addr: work.room,
                        session: None,
                        effort: None,
                        model: None,
                        policy,
                        origin: None,
                        parent: Some(run),
                        succession: None,
                        taint: taint.clone(),
                        dispatched_by: by,
                    },
                    work.task,
                    work.goal,
                    owing.child(room.clone()),
                )
            });
            if let Err(err) = started {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "collab::delegate",
                    &format!(
                        "{} handed work to {} and it could not start: {}",
                        room.as_str(),
                        child.as_str(),
                        err.subject()
                    ),
                );
            }
        }
        Ok(())
    }

    /// Ends the hand-over for a run that has come home: anything it asked
    /// for that the city has not seen yet starts now, and a graph it
    /// registered is closed when its ending says so.
    ///
    /// # Errors
    /// As [`RunWorker::hand_over_at_call`].
    pub(in crate::worker) fn end_hand_over(
        &mut self,
        run: RunId,
        after: GraphAfter,
    ) -> Result<(), AxError> {
        self.hand_over_at_call(run)?;
        let Some(handing) = self.collaborating.handing.remove(&run) else {
            return Ok(());
        };
        match (handing.graph, after) {
            (Graph::Registered, GraphAfter::Closed) => {
                self.collaborating.workshops.remove(&handing.room);
            }
            (Graph::Registered, GraphAfter::Open)
            | (Graph::NotLaidOut, GraphAfter::Open | GraphAfter::Closed) => {}
        }
        Ok(())
    }
}
