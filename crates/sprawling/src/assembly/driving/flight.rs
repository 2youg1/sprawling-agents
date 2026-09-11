// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every run in a lane right now, and what the city owes each one when
//! it comes home.
//!
//! **One table, not two.** The lanes, the crossing those lanes write
//! history through, and what each driving run is carrying all live
//! here, so a city has one answer to "how many runs am I driving" and
//! one crossing to serve. Two tables would be two ceilings and two
//! crossings: a lane's append would wait on whichever thread happened
//! to be serving the other one (sprawling-SPEC.md 8-46-2).
//!
//! Nothing here decides anything. Which runs to start is the caller's,
//! and what a landed run means is `Owed`'s — this module only keeps the
//! two together for as long as a drive takes.

use std::collections::BTreeMap;
use std::time::Duration;

use kernel::{Address, AxCode, AxError, NodeId, RunId};

use super::super::{Continuation, Driven, RunWorker};
use super::{Driving, lane::DriveContext};
use crate::serving::pool::{Arrival, DRIVING_LANES, DrivingPool};
use crate::serving::relay::{Relay, RelayGate};

/// How long the accounting thread waits for a run to come home before
/// it serves the crossing again.
///
/// Short because the crossing is what lets a lane make progress at all:
/// a long wait here is a lane blocked on an append nobody has looked at
/// yet.
pub(crate) const LOOK_AGAIN: Duration = Duration::from_millis(1);

/// What the city still owes once a run has landed.
///
/// The two entrances that put a run in a lane differ only here, which
/// is why the rest of a dispatch has one path rather than two.
pub(in crate::assembly) enum Owed {
    /// A person asked for this dispatch: a refusal has an address to go
    /// back to, and whoever the run spoke to answers next.
    Asked(channels::Reply),
    /// A pursuit took this plan row. If the row is still ready when the
    /// run comes home, the pursuit stops rather than taking it again.
    Row { addr: Address, node: NodeId },
}

/// One run in a lane: everything the city does once the drive is home,
/// and what that landing is owed.
struct InLane {
    continuation: Continuation,
    owed: Owed,
}

/// What one pass over the crossing and the lanes did.
///
/// Exhaustive because the two callers act on different parts of it: the
/// worker loop only needs to know that it made progress, and a pursuit
/// needs to know which of its own rows came back.
pub(crate) enum Landed {
    /// No run came home inside the wait. Relay requests were still
    /// served, which is the half that keeps the lanes moving.
    Nothing,
    /// A dispatch somebody asked for has landed.
    Asked,
    /// A pursuit's row has landed.
    Row { addr: Address, node: NodeId },
}

/// The lanes, the crossing, and what each driving run is carrying.
pub(in crate::assembly) struct Flight {
    pool: DrivingPool,
    gate: RelayGate,
    driving: BTreeMap<RunId, InLane>,
}

impl Flight {
    pub(in crate::assembly) fn open() -> Flight {
        Flight {
            pool: DrivingPool::open(DRIVING_LANES),
            gate: RelayGate::open(),
            driving: BTreeMap::new(),
        }
    }

    /// How many runs are driving right now, whoever started them.
    pub(in crate::assembly) fn in_flight(&self) -> u32 {
        self.pool.in_flight()
    }

    /// Whether every lane is taken. The concurrency wall a caller reads
    /// before it prepares work it cannot start.
    pub(in crate::assembly) fn full(&self) -> bool {
        self.pool.full()
    }

    /// The plan rows one pursuit has in lanes: how many, and which
    /// nodes.
    ///
    /// A pursuit waits for its own rows and for nothing else. Runs a
    /// person dispatched are landed by the worker loop, and a pursuit
    /// that waited for them would tie two unrelated pieces of work
    /// together.
    pub(in crate::assembly) fn rows_of(
        &self,
        addr: &Address,
    ) -> std::collections::BTreeSet<NodeId> {
        self.driving
            .values()
            .filter_map(|lane| match &lane.owed {
                Owed::Row { addr: at, node } if at == addr => Some(node.clone()),
                Owed::Row { .. } | Owed::Asked(_) => None,
            })
            .collect()
    }

    /// Takes one prepared drive into a lane of its own.
    ///
    /// # Errors
    /// Propagates the pool's refusal to start a thread, and its refusal
    /// of a run that is already driving.
    fn take(
        &mut self,
        driving: Driving,
        context: DriveContext,
        carried: InLane,
    ) -> Result<RunId, AxError> {
        let run = driving.run_id;
        self.pool.start(driving, self.issue(), context)?;
        self.driving.insert(run, carried);
        Ok(run)
    }

    /// One write face for one lane.
    fn issue(&self) -> Relay {
        self.gate.issue()
    }

    /// The next run home with what the city owed it, or `None` if none
    /// arrived inside `wait`.
    ///
    /// # Errors
    /// Propagates a lane that ended without saying so, and refuses a
    /// run the table never took — a lane the pool started that this
    /// table does not know is a defect in [`Self::take`], and landing
    /// it blind would settle a run against nothing.
    fn arrived(&mut self, wait: Duration) -> Option<Result<Home, AxError>> {
        let Arrival { run, driven } = match self.pool.arrived(wait)? {
            Ok(arrival) => arrival,
            Err(err) => return Some(Err(err)),
        };
        let Some(InLane { continuation, owed }) = self.driving.remove(&run) else {
            return Some(Err(AxError::failure(
                AxCode::StorageFatal,
                "land a run that came home",
                format!("{run} was driving and the city was carrying nothing for it"),
            )
            .with_recovery("report this: a lane is entered and recorded together")));
        };
        Some(Ok(Home {
            driven,
            continuation,
            owed,
        }))
    }
}

/// One run home from its lane, with everything the city kept for it.
struct Home {
    driven: Result<Driven, AxError>,
    continuation: Continuation,
    owed: Owed,
}

impl RunWorker {
    /// Writes every relay request already waiting, and lands at most one
    /// run that came home.
    ///
    /// **The crossing is served first**, because a run that has already
    /// been paid for must not queue behind one that has not started
    /// (sprawling-SPEC.md 8-42-2). Landing happens in arrival order, on
    /// this thread, which is what makes "execute in parallel, account in
    /// series" a fact about the code.
    ///
    /// # Errors
    /// Propagates a lane that ended without saying so, and every failure
    /// of landing a run. A dispatch somebody asked for is *also* handed
    /// its refusal, because the caller of this function is a loop and
    /// the person who asked is not in it.
    pub(crate) fn serve_flight(&mut self, wait: Duration) -> Result<Landed, AxError> {
        self.flight.gate.serve_waiting(&mut self.ledger);
        let Some(arrival) = self.flight.arrived(wait) else {
            return Ok(Landed::Nothing);
        };
        let Home {
            driven,
            continuation,
            owed,
        } = arrival?;
        match owed {
            Owed::Asked(reply) => match self.land(continuation, driven) {
                Ok(_) => {
                    // Whoever this run spoke to answers next, and
                    // whoever they speak to after that. The person
                    // asked for one dispatch; what follows is the
                    // conversation it started.
                    self.answer_knocks();
                    Ok(Landed::Asked)
                }
                Err(err) => {
                    self.hand_back(&reply, err.clone());
                    Err(err)
                }
            },
            Owed::Row { addr, node } => {
                self.land(continuation, driven)?;
                Ok(Landed::Row { addr, node })
            }
        }
    }

    /// Whether any run is driving right now.
    pub(crate) fn driving(&self) -> bool {
        self.flight.in_flight() > 0
    }

    /// Serves the crossing and lands runs until no lane is left.
    ///
    /// What a closing city does before it writes its handoff: a lane
    /// abandoned while it waits on an append loses lines the city had
    /// already told it were durable.
    ///
    /// # Errors
    /// Propagates the first landing that fails. The lanes still going
    /// are left to the caller, which is closing anyway.
    pub(crate) fn land_the_rest(&mut self) -> Result<(), AxError> {
        while self.driving() {
            self.serve_flight(LOOK_AGAIN)?;
        }
        Ok(())
    }

    /// Prepares one dispatch on this thread and takes the drive into a
    /// lane, so the desk is free again before the run has finished.
    ///
    /// This is the person's entrance. Everything the city writes before
    /// a model is called — agreeing to the work, opening the room,
    /// writing the brief, standing the run up — happens here, on the
    /// accounting thread, and is finished by the time this returns. What
    /// continues is the run, not the command, which is why the
    /// idempotency key settles at take-off (sprawling-SPEC.md 8-46-2).
    ///
    /// # Errors
    /// Propagates every refusal a dispatch can owe before it costs
    /// anything, and the pool's refusal to start a lane.
    pub(in crate::assembly) fn dispatch_into_lane(
        &mut self,
        at: super::super::Assignment,
        task: String,
        goal: String,
        reply: channels::Reply,
    ) -> Result<RunId, AxError> {
        let (driving, continuation) = self.prepare_dispatch(at, task, goal)?;
        let context = self.drive_context();
        self.flight.take(
            driving,
            context,
            InLane {
                continuation,
                owed: Owed::Asked(reply),
            },
        )
    }

    /// Puts a prepared drive into a lane on a pursuit's behalf.
    ///
    /// # Errors
    /// Propagates the pool's refusal to start a lane.
    pub(in crate::assembly) fn take_row_into_lane(
        &mut self,
        driving: Driving,
        addr: Address,
        node: NodeId,
        continuation: Continuation,
    ) -> Result<RunId, AxError> {
        let context = self.drive_context();
        self.flight.take(
            driving,
            context,
            InLane {
                continuation,
                owed: Owed::Row { addr, node },
            },
        )
    }
}
