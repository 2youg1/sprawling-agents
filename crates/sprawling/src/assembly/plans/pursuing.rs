// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole ready set, driven at once: which nodes are in somebody's
//! hands, which lanes they are in, and where each one lands.
//!
//! **Execute in parallel, account in series** (ARCHITECTURE section 10,
//! rule 5). Every node of the ready set gets a lane of its own; every
//! line those lanes write crosses back to this thread through the
//! relay, and every run that comes home is landed here, in the order it
//! arrives (sprawling-SPEC.md 8-46-4).

use std::time::Duration;

use kernel::{Address, AxError, NodeId, RunId};

use super::super::{Assignment, RunWorker};
use crate::serving::pool::{Arrival, DRIVING_LANES, DrivingPool};
use crate::serving::relay::RelayGate;

/// How long the accounting thread waits for a run to come home before
/// it serves the relay again.
///
/// Short because the relay is what lets a lane make progress at all: a
/// long wait here is a lane blocked on an append this thread has not
/// looked at yet.
const LOOK_AGAIN: Duration = Duration::from_millis(1);

/// One pursuit in progress: the lanes, the crossing they write through,
/// and what each lane is carrying.
///
/// It is a local of one `pursue` call rather than a field of the
/// worker, which is what keeps this card inside the pursuit: the
/// worker's own command loop still runs one command at a time
/// (sprawling-SPEC.md 8-46-2).
struct Pursuing {
    pool: DrivingPool,
    gate: RelayGate,
    /// What each driving run is: the plan node it took, and everything
    /// the city has to do once it comes home.
    taken: std::collections::BTreeMap<RunId, (NodeId, super::super::Continuation)>,
    /// Set when a run came home leaving its node ready. The pursuit
    /// stops taking new work rather than dispatching that node again,
    /// which is how this loop has always been kept from spinning.
    stalled: bool,
}

impl Pursuing {
    fn open() -> Pursuing {
        Pursuing {
            pool: DrivingPool::open(DRIVING_LANES),
            gate: RelayGate::open(),
            taken: std::collections::BTreeMap::new(),
            stalled: false,
        }
    }

    /// The nodes somebody is already working on. They are not ready:
    /// ready means a run could take this node now, and a node in a
    /// lane has been taken.
    fn busy(&self) -> std::collections::BTreeSet<NodeId> {
        self.taken.values().map(|(node, _)| node.clone()).collect()
    }
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
        let mut going = Pursuing::open();
        loop {
            self.take_ready_work(addr, &mut going)?;
            if going.pool.in_flight() == 0 {
                return Ok(());
            }
            // The relay first, and a run home second: a lane that has
            // already been paid for must not queue behind anything
            // (sprawling-SPEC.md 8-42-2).
            self.serve_relay(&going.gate);
            let Some(arrival) = going.pool.arrived(LOOK_AGAIN) else {
                continue;
            };
            let Arrival { run, driven } = arrival?;
            let Some((node, continuation)) = going.taken.remove(&run) else {
                continue;
            };
            self.land(continuation, driven)?;
            // A run that came home leaving its node exactly as it found
            // it would be dispatched again for ever, so the check is on
            // the ready set rather than on a counter.
            if self.ready_in(addr).contains(&node) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "kernel::pursuit",
                    &format!(
                        "{node} is still ready after a run took it; the pursuit stops rather \
                         than dispatching it again"
                    ),
                );
                going.stalled = true;
            }
        }
    }

    /// Starts a run for every ready node there is a free lane for.
    ///
    /// Nodes already in a lane are not in the ready set it asks about:
    /// ready means a run could take this node now, and one already
    /// taken could not be taken twice.
    fn take_ready_work(&mut self, addr: &Address, going: &mut Pursuing) -> Result<(), AxError> {
        while !going.stalled && !going.pool.full() {
            let Some(state) = self.pursuits.get(addr).map(kernel::Pursuit::state) else {
                return Ok(());
            };
            let busy = going.busy();
            let ready: Vec<NodeId> = self
                .ready_in(addr)
                .into_iter()
                .filter(|node| !busy.contains(node))
                .collect();
            let kernel::PursuitVerdict::Work { next } =
                kernel::observe_pursuit(state, &ready, going.pool.in_flight())
            else {
                return Ok(());
            };
            let (Some(item), Some(goal)) = (
                self.plan_item(addr, &next),
                self.pursuits.get(addr).map(|held| held.goal().to_owned()),
            ) else {
                return Ok(());
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
            let run = driving.run_id;
            going
                .pool
                .start(driving, going.gate.issue(), self.drive_context())?;
            going.taken.insert(run, (next, continuation));
        }
        Ok(())
    }
}
