// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{Address, AxError, RunId};

use super::super::dispatching::handback::Unstarted;
use super::super::{Assignment, Continuation, Owing, RunWorker, held};

const DELEGATE_DESK: &str = "read the delegate desk";
const WORKSHOP_DESK: &str = "read the workshop desk";

pub(in crate::worker) struct Handing {
    room: Address,
    delegates: Arc<Mutex<collab::DelegateDesk>>,
    workshop: Arc<Mutex<collab::WorkshopDesk>>,
    policy: kernel::RunPolicy,
    taint: kernel::TaintSet,

    owing: Owing,
    graph: Graph,
}

#[derive(Clone, Copy)]
enum Graph {
    NotLaidOut,
    Registered,
}

#[derive(Clone, Copy)]
pub(in crate::worker) enum GraphAfter {

    Open,

    Closed,
}

impl GraphAfter {

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

impl Continuation {

    pub(in crate::worker) fn handing(&self, at: &Assignment, owing: &Owing) -> Option<Handing> {
        match self {
            Continuation::Model(lent) => Some(Handing::new(
                at,
                std::sync::Arc::clone(&lent.desks.delegates),
                std::sync::Arc::clone(&lent.desks.workshop),
                owing,
            )),
            Continuation::Harness => None,
        }
    }
}

impl RunWorker {

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

        let (first, fresh) = {
            let mut desk = held(&handing.delegates, DELEGATE_DESK)?;
            let fresh = desk.hand_over();
            (desk.asked().len().saturating_sub(fresh.len()), fresh)
        };
        let (room, policy, taint) = (handing.room.clone(), handing.policy, handing.taint.clone());
        let chain = handing.owing.knock_chain().clone();
        let owing = handing.owing.child(room.clone());
        for (offset, work) in fresh.into_iter().enumerate() {
            let child = work.room.clone();
            self.note(
                runtime::diagnostics::Level::Effect,
                "collab::delegate",
                &format!("{} handed work to {}", room.as_str(), child.as_str()),
            );

            let started = kernel::event::Who::resident(room.clone()).and_then(|by| {
                self.dispatch_into_lane(
                    Assignment {
                        addr: work.room,
                        session: None,
                        effort: None,
                        difficulty: work.difficulty,
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
            if let Err(refusal) = started {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "collab::delegate",
                    &format!(
                        "{} handed work to {} and it could not start: {}",
                        room.as_str(),
                        child.as_str(),
                        refusal.subject()
                    ),
                );
                let unstarted = Unstarted {
                    parent: &room,
                    asker: run,
                    place: first.saturating_add(offset),
                    child: &child,
                    refusal: &refusal,
                };
                if let Err(err) = self.deliver_unstarted(unstarted, policy, &chain) {
                    self.note(
                        runtime::diagnostics::Level::Refuse,
                        "collab::delegate",
                        &format!(
                            "{} was not told that {} could not start: {}",
                            room.as_str(),
                            child.as_str(),
                            err.subject()
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    pub(in crate::worker) fn handing_room(&self, run: RunId) -> Option<&Address> {
        self.collaborating
            .handing
            .get(&run)
            .map(|handing| &handing.room)
    }

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
