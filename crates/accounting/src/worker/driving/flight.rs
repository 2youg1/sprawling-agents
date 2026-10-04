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
//! to be serving the other one (`crates/sprawling/Spec.lean` §8-46-2).
//!
//! Nothing here decides anything. Which runs to start is the caller's,
//! and what a landed run means is `Owed`'s — this module only keeps the
//! two together for as long as a drive takes.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc;

use kernel::{Address, AxCode, AxError, NodeId, RunId};

use super::super::{Continuation, Owed, Owing, RunWorker};
use super::lane::DriveContext;
use crate::worker::booking::OpenClaims;
use crate::worker::dispatching::preparing::{Flown, Staged};
use crate::worker::pool::{Arrival, DrivingPool};
use crate::worker::relay::{Drained, Patience, Relay, RelayGate, Wake};

/// One run in a lane: everything the city does once the drive is home,
/// and what that landing is owed.
pub(super) struct InLane {
    pub(super) continuation: Continuation,
    pub(super) owing: Owing,
}

/// Who a driving run speaks as when a signal it sends knocks on an
/// empty room: its room, the policy it runs under, and its place in the
/// conversation. Kept beside the run for as long as it drives, so the
/// accounting thread knocks when it delivers the signal rather than
/// when the sender lands (collab D7).
pub(in crate::worker) struct Speaker<'a> {
    pub(in crate::worker) room: &'a Address,
    pub(in crate::worker) policy: kernel::RunPolicy,
    pub(in crate::worker) chain: &'a super::super::KnockChain,
}

/// One driving run: what its landing needs, and where it stands.
struct Driving {
    lane: InLane,
    room: Address,
    policy: kernel::RunPolicy,
}

/// What one pass over the crossing and the lanes did.
///
/// Exhaustive because the two callers act on different parts of it: the
/// worker loop only needs to know that it made progress, and a pursuit
/// needs to know which of its own rows came back.
#[derive(Debug)]
pub(crate) enum Landed {
    /// No run came home in this look. Relay requests were still
    /// served, which is the half that keeps the lanes moving.
    Nothing,
    /// A pursuit's row has landed.
    Row { addr: Address, node: NodeId },
    /// A run landed that no pursuit is waiting for: a person's
    /// dispatch, a job the city started itself, or work handed down.
    /// The worker loop needs to know only that it made progress.
    Elsewhere,
}

/// The lanes, the crossing, and what each driving run is carrying.
pub(in crate::worker) struct Flight {
    pool: DrivingPool,
    pub(in crate::worker) gate: RelayGate,
    driving: BTreeMap<RunId, Driving>,
    /// Runs home that one drain found beyond the one it landed, in
    /// arrival order. Kept rather than re-queued, so arrival order is
    /// landing order.
    homes: VecDeque<Arrival>,
    /// What is still running while the runs go on. One table per city,
    /// and every `exec` gets a handle onto it, so `halt` reaches a
    /// command without knowing which tool started it.
    pub(in crate::worker) backlog: runtime::Backlog,
}

impl Flight {
    /// `read_memory` is where the pool reads how much memory is free
    /// before it starts a run (`crates/sprawling/Spec.lean` §8-46-3).
    /// `monotonic` is the clock every wait the crossing records is read
    /// off (`crates/sprawling/spec/Accounting/Worker.lean` §8-98).
    /// `seat_lane` is called at the top of every lane; `backlog` carries
    /// the shares and affinity requested for each run's commands.
    pub(in crate::worker) fn open(
        read_memory: fn() -> crate::worker::pool::Memory,
        monotonic: fn() -> std::time::Instant,
        seat_lane: crate::worker::hands::SeatLane,
        backlog: runtime::Backlog,
    ) -> Flight {
        let gate = RelayGate::open(monotonic);
        let lanes = crate::worker::pool::LaneHands {
            read_memory,
            monotonic,
            seat_lane,
        };
        Flight {
            pool: DrivingPool::open(gate.bell(), lanes, gate.health()),
            gate,
            driving: BTreeMap::new(),
            homes: VecDeque::new(),
            backlog,
        }
    }

    /// How many runs are driving right now, whoever started them.
    pub(in crate::worker) fn in_flight(&self) -> u32 {
        self.pool.in_flight()
    }

    /// Whether a new run waits: memory is tight while a run is driving.
    /// The concurrency wall a caller reads before it prepares work it
    /// cannot start; the memory is read here, at the moment it decides.
    pub(in crate::worker) fn full(&self) -> bool {
        self.pool.full()
    }

    /// The plan rows one pursuit has in lanes: how many, and which
    /// nodes.
    ///
    /// A pursuit waits for its own rows and for nothing else. Runs a
    /// person dispatched are landed by the worker loop, and a pursuit
    /// that waited for them would tie two unrelated pieces of work
    /// together.
    pub(in crate::worker) fn rows_of(&self, addr: &Address) -> std::collections::BTreeSet<NodeId> {
        self.driving
            .values()
            .filter_map(|driving| match driving.lane.owing.owed() {
                Owed::Row { addr: at, node } if at == addr => Some(node.clone()),
                Owed::Row { .. } | Owed::Asked | Owed::Unasked(_) | Owed::Child { .. } => None,
            })
            .collect()
    }

    /// Takes one staged dispatch into a lane, which prepares and drives it.
    ///
    /// # Errors
    /// Propagates the pool's refusal to start a thread, and its refusal
    /// of a run that is already driving.
    pub(super) fn take(
        &mut self,
        staged: Staged,
        context: DriveContext,
        carried: InLane,
    ) -> Result<RunId, AxError> {
        let run = staged.run_id();
        let at = staged.assignment();
        let (room, policy) = (at.addr.clone(), at.policy);
        self.pool.start(staged, self.issue(), context)?;
        self.driving.insert(
            run,
            Driving {
                lane: carried,
                room,
                policy,
            },
        );
        Ok(run)
    }

    /// Who `run` speaks as, while it drives.
    pub(in crate::worker) fn speaker(&self, run: RunId) -> Option<Speaker<'_>> {
        self.driving.get(&run).map(|driving| Speaker {
            room: &driving.room,
            policy: driving.policy,
            chain: driving.lane.owing.knock_chain(),
        })
    }

    /// One write face for one lane.
    fn issue(&self) -> Relay {
        self.gate.issue()
    }

    /// The next run home that a drain already found, with what the city
    /// owed it, or `None` if there is none.
    ///
    /// # Errors
    /// Propagates a lane that ended without saying so, and refuses a
    /// run the table never took — a lane the pool started that this
    /// table does not know is a defect in [`Self::take`], and landing
    /// it blind would settle a run against nothing.
    fn arrived(&mut self) -> Option<Result<Home, AxError>> {
        let Arrival { run, flown } = match self.pool.landed(self.homes.pop_front()?) {
            Ok(arrival) => arrival,
            Err(err) => return Some(Err(err)),
        };
        let Some(Driving {
            lane: InLane {
                continuation,
                owing,
            },
            ..
        }) = self.driving.remove(&run)
        else {
            return Some(Err(AxError::failure(
                AxCode::StorageFatal,
                "land a run that came home",
                format!("{run} was driving and the city was carrying nothing for it"),
            )
            .with_recovery("report this: a lane is entered and recorded together")));
        };
        // Its landing runs on this thread before the queue is served
        // again, so no claim is answered between the release and the
        // plan on disk saying how the run's nodes ended.
        Some(Ok(Home {
            flown,
            continuation,
            owing,
            open_claims: self.gate.booked.release(run),
        }))
    }
}

/// One run home from its lane, with everything the city kept for it.
struct Home {
    flown: Flown,
    continuation: Continuation,
    owing: Owing,
    open_claims: OpenClaims,
}

impl RunWorker {
    /// Waits as `patience` allows for the one queue to wake, writes every
    /// relay request it holds, and lands at most one run that came home.
    /// A run already found home and not yet landed makes the look
    /// [`Patience::Now`], because it is work that is already waiting.
    ///
    /// **The crossing is served first**, because a run that has already
    /// been paid for must not queue behind one that has not started
    /// (`crates/sprawling/Spec.lean` §8-42-2). Landing happens in arrival order, on
    /// this thread, which is what makes "execute in parallel, account in
    /// series" a fact about the code.
    ///
    /// # Errors
    /// Propagates a lane that ended without saying so, and every failure
    /// of landing a run. A dispatch somebody asked for is *also* handed
    /// its refusal, because the caller of this function is a loop and
    /// the person who asked is not in it.
    pub(in crate::worker) fn serve_flight(
        &mut self,
        patience: Patience,
    ) -> Result<Landed, AxError> {
        let patience = if self.flight.homes.is_empty() {
            patience
        } else {
            Patience::Now
        };
        let Drained { written, goals } =
            self.flight
                .gate
                .serve(patience, &mut self.ledger, &mut self.flight.homes);
        self.show_relayed(written);
        for ask in goals {
            self.answer_goal(ask);
        }
        let Some(arrival) = self.flight.arrived() else {
            return Ok(Landed::Nothing);
        };
        // A lane has just come home, so the work waiting for one starts
        // before this run is landed (`crates/sprawling/Spec.lean` §8-46-2).
        for (run, err) in self.flight.pool.start_waiting() {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "accounting::worker",
                &format!(
                    "{run} waited for a lane and could not start: {}",
                    err.subject()
                ),
            );
            if let Some(driving) = self.flight.driving.remove(&run) {
                self.hand_back(&driving.lane.owing.reply(), err);
            }
        }
        let Home {
            flown,
            continuation,
            owing,
            mut open_claims,
        } = arrival?;
        // Read before the obligation is spent, because a successor
        // takes it over and the refusal below still has to reach the
        // person who asked for the run this one replaced.
        let reply = owing.reply();
        let landed = self.land(continuation, flown, owing, &mut open_claims);
        let given_back = self.give_back_claims(open_claims);
        match landed {
            Ok(landed) => {
                given_back?;
                self.advance_pursuits(&landed);
                Ok(landed)
            }
            Err(err) => {
                if let Err(lost) = given_back {
                    self.note(
                        runtime::diagnostics::Level::Refuse,
                        "crate::worker::booking",
                        &format!("a claim of a run whose landing failed stays open: {lost}"),
                    );
                }
                self.hand_back(&reply, err.clone());
                Err(err)
            }
        }
    }

    /// Writes the lines that give back every node a run still had booked
    /// when its landing ended without settling its plan: the claim went
    /// on the history at call time, so only this closes it
    /// (`crates/sprawling/Spec.lean` §8-42-8).
    ///
    /// # Errors
    /// Propagates the first line the ledger refuses.
    fn give_back_claims(&mut self, open_claims: OpenClaims) -> Result<(), AxError> {
        let (run, put_backs) = open_claims.owed();
        put_backs
            .into_iter()
            .try_for_each(|line| self.record_for(run, line))
    }

    /// A write face issued by the same gate the lanes write through, for
    /// an instrument that times the crossing from outside a lane
    /// (`crates/sprawling/Spec.lean` §8-84).
    #[cfg(test)]
    pub(in crate::worker) fn measuring_relay(&self) -> Relay {
        self.flight.issue()
    }

    /// A sender onto the accounting thread's one queue, for the desk
    /// this thread attends.
    pub(in crate::worker) fn bell(&self) -> mpsc::Sender<Wake> {
        self.flight.gate.bell()
    }

    /// Whether any run is driving right now.
    pub fn driving(&self) -> bool {
        self.flight.in_flight() > 0
    }

    /// Whether `run` is in a lane right now, and so reads its own Cancel
    /// and Steer off the desk at its safe points.
    pub(in crate::worker) fn drives(&self, run: RunId) -> bool {
        self.flight.driving.contains_key(&run)
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
    pub(in crate::worker) fn land_the_rest(&mut self) -> Result<(), AxError> {
        while self.driving() {
            self.serve_flight(Patience::Unbounded)?;
        }
        Ok(())
    }
}
