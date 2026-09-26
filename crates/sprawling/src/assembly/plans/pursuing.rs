// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole ready set, driven at once: which nodes are in somebody's
//! hands, and where each one lands.
//!
//! **Execute in parallel, account in series** (ARCHITECTURE section 10,
//! rule 5). Every node of the ready set gets a lane of its own; every
//! line those lanes write crosses back to this thread, and every run
//! that comes home is landed by `serve_flight`, in the order it
//! arrives, which then moves every pursuit on (sprawling-SPEC.md
//! 8-46-4).
//!
//! The lanes are the city's, not this pursuit's: a pursuit takes rows
//! into the same table a person's dispatch goes into, so the number of
//! runs a city drives at once has one answer (sprawling-SPEC.md 8-46-2).

use kernel::{Address, AxError, NodeId};

use super::super::{Assignment, Landed, RunWorker};

impl RunWorker {
    /// Moves every pursuit on after a run has come home.
    ///
    /// **It terminates because a node leaves the ready set when a lane
    /// takes it, and comes back only if the run that took it left it
    /// alone** — and that case pauses the pursuit rather than
    /// re-dispatching, so the ledger and the page both say it stopped.
    /// Splitting a branch adds work, which is the city finding more to
    /// do rather than looping.
    ///
    /// Any landing may have freed a lane or made a node ready, so every
    /// pursuit is asked, not only the one whose row came home. A pursuit
    /// that cannot take work is noted rather than failing the landing
    /// that happened to come before it: the two have no cause in common.
    pub(in crate::assembly) fn advance_pursuits(&mut self, landed: &Landed) {
        if let Landed::Row { addr, node } = landed
            && self.ready_in(addr).contains(node)
        {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "kernel::pursuit",
                &format!(
                    "{node} is still ready after a run took it; the pursuit pauses rather than \
                     dispatching it again"
                ),
            );
            if let Err(err) = self.set_pursuit(addr, channels::PursuitStep::Pause) {
                self.pursuit_refused(addr, &err);
            }
        }
        let pursuing: Vec<Address> = self.planning.pursuits.keys().cloned().collect();
        for addr in pursuing {
            if let Err(err) = self.pursue(&addr) {
                self.pursuit_refused(&addr, &err);
            }
        }
    }

    /// Notes a pursuit that could not move on, against its building.
    fn pursuit_refused(&mut self, addr: &Address, err: &AxError) {
        self.note(
            runtime::diagnostics::Level::Refuse,
            "kernel::pursuit",
            &format!("{} could not take work: {}", addr.as_str(), err.subject()),
        );
    }

    /// Starts a run for every ready node there is a free lane for, and
    /// returns.
    ///
    /// **Moved on by events, not by a loop.** The rows it starts land
    /// through `serve_flight` like any other run, and each landing calls
    /// [`Self::advance_pursuits`], so the desk is free the moment this
    /// returns: a loop here held `Pause`, `Halt` and every other command
    /// out until the pursuit's last row came home (sprawling-SPEC.md
    /// 8-46-4).
    ///
    /// Nodes already in a lane are not in the ready set it asks about:
    /// ready means a run could take this node now, and one already
    /// taken could not be taken twice. The verdict is
    /// `kernel::pursuit`'s and is not re-derived here.
    ///
    /// # Errors
    /// Propagates a dispatch that could not be prepared and a lane that
    /// could not be started.
    pub(super) fn pursue(&mut self, addr: &Address) -> Result<(), AxError> {
        while !self.flight.full() {
            let Some(state) = self.planning.pursuits.get(addr).map(kernel::Pursuit::state) else {
                return Ok(());
            };
            let busy = self.flight.rows_of(addr);
            let ready: Vec<NodeId> = self
                .ready_in(addr)
                .into_iter()
                .filter(|node| !busy.contains(node))
                .collect();
            let kernel::PursuitVerdict::Work { next } =
                kernel::pursuit::observe(state, &ready, self.flight.in_flight())
            else {
                return Ok(());
            };
            let (Some(item), Some(goal)) = (
                self.plan_item(addr, &next),
                self.planning
                    .pursuits
                    .get(addr)
                    .map(|held| held.goal().to_owned()),
            ) else {
                return Ok(());
            };
            self.note(
                runtime::diagnostics::Level::Effect,
                "kernel::pursuit",
                &format!("{} takes {next}: {item}", addr.as_str()),
            );
            let (staged, continuation) = self.stage_dispatch(
                Assignment {
                    addr: addr.clone(),
                    session: None,
                    effort: None,
                    model: None,
                    mode: kernel::Mode::PlanGoal,
                    parent: None,
                    origin: None,
                    succession: None,
                    taint: kernel::TaintSet::empty(),
                    dispatched_by: kernel::event::Who::City,
                },
                format!("Plan node {next}: {item}"),
                goal,
            )?;
            self.take_row_into_lane(staged, addr.clone(), next, continuation)?;
        }
        Ok(())
    }
}
