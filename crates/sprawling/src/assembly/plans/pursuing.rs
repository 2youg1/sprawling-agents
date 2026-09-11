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
//! that comes home is landed here, in the order it arrives
//! (sprawling-SPEC.md 8-46-4).
//!
//! The lanes are the city's, not this pursuit's: a pursuit takes rows
//! into the same table a person's dispatch goes into, so the number of
//! runs a city drives at once has one answer (sprawling-SPEC.md 8-46-2).

use kernel::{Address, AxError, NodeId};

use super::super::{Assignment, LOOK_AGAIN, Landed, RunWorker};

/// Why one pass over the ready set stopped taking work.
///
/// The difference decides whether the pursuit is over: nothing left to
/// take and nothing of its own in a lane is finished, while a full city
/// still has this pursuit's work ahead of it.
enum Taking {
    /// `kernel::pursuit` says there is nothing to take right now.
    Nothing,
    /// Every lane in the city is taken, and ready work is still waiting.
    LanesFull,
}

impl RunWorker {
    /// Takes ready work for as long as a pursuit says to, driving every
    /// node of the ready set at once.
    ///
    /// **It terminates because a node leaves the ready set when a lane
    /// takes it, and comes back only if the run that took it left it
    /// alone** — and that case stops the pursuit rather than
    /// re-dispatching. Splitting a branch adds work, which is the city
    /// finding more to do rather than looping.
    ///
    /// The verdict is `kernel::pursuit`'s and is not re-derived here:
    /// what "there is nothing left to do" means has one authority, and
    /// this is the first caller able to tell it the truth about how many
    /// runs are going.
    ///
    /// # Errors
    /// Propagates a dispatch that could not be prepared, a lane that
    /// could not be started, and every failure of landing a run.
    pub(super) fn pursue(&mut self, addr: &Address) -> Result<(), AxError> {
        let mut stalled = false;
        loop {
            let taking = if stalled {
                Taking::Nothing
            } else {
                self.take_ready_work(addr)?
            };
            if matches!(taking, Taking::Nothing) && self.flight.rows_of(addr).is_empty() {
                return Ok(());
            }
            // A run home may be this pursuit's row or somebody else's
            // dispatch; both land here, and only the first one changes
            // what this loop does next.
            let Landed::Row { addr: at, node } = self.serve_flight(LOOK_AGAIN)? else {
                continue;
            };
            // A run that came home leaving its node exactly as it found
            // it would be dispatched again for ever, so the check is on
            // the ready set rather than on a counter.
            if at == *addr && self.ready_in(addr).contains(&node) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "kernel::pursuit",
                    &format!(
                        "{node} is still ready after a run took it; the pursuit stops rather \
                         than dispatching it again"
                    ),
                );
                stalled = true;
            }
        }
    }

    /// Starts a run for every ready node there is a free lane for.
    ///
    /// Nodes already in a lane are not in the ready set it asks about:
    /// ready means a run could take this node now, and one already
    /// taken could not be taken twice.
    fn take_ready_work(&mut self, addr: &Address) -> Result<Taking, AxError> {
        while !self.flight.full() {
            let Some(state) = self.pursuits.get(addr).map(kernel::Pursuit::state) else {
                return Ok(Taking::Nothing);
            };
            let busy = self.flight.rows_of(addr);
            let ready: Vec<NodeId> = self
                .ready_in(addr)
                .into_iter()
                .filter(|node| !busy.contains(node))
                .collect();
            let kernel::PursuitVerdict::Work { next } =
                kernel::observe_pursuit(state, &ready, self.flight.in_flight())
            else {
                return Ok(Taking::Nothing);
            };
            let (Some(item), Some(goal)) = (
                self.plan_item(addr, &next),
                self.pursuits.get(addr).map(|held| held.goal().to_owned()),
            ) else {
                return Ok(Taking::Nothing);
            };
            self.note(
                runtime::diagnostics::Level::Effect,
                "kernel::pursuit",
                &format!("{} takes {next}: {item}", addr.as_str()),
            );
            let (driving, continuation) = self.prepare_dispatch(
                Assignment {
                    addr: addr.clone(),
                    session: None,
                    effort: None,
                    mode: runtime::Mode::PlanGoal,
                    parent: None,
                    succession: None,
                },
                format!("Plan node {next}: {item}"),
                goal,
            )?;
            self.take_row_into_lane(driving, addr.clone(), next, continuation)?;
        }
        Ok(Taking::LanesFull)
    }
}
