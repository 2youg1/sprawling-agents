// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc;

use kernel::{Address, AxCode, AxError, NodeId, RunId};

use super::super::{Continuation, Owed, Owing, RunWorker};
use super::lane::DriveContext;
use crate::worker::booking::OpenClaims;
use crate::worker::dispatching::preparing::{Flown, Staged};
use crate::worker::errand::Errands;
use crate::worker::pool::{Arrival, DrivingPool};
use crate::worker::relay::{Drained, Patience, Relay, RelayGate, Wake};

pub(super) struct InLane {
    pub(super) continuation: Continuation,
    pub(super) owing: Owing,
}

pub(in crate::worker) struct Speaker<'a> {
    pub(in crate::worker) room: &'a Address,
    pub(in crate::worker) policy: kernel::RunPolicy,
    pub(in crate::worker) chain: &'a super::super::KnockChain,
}

struct Driving {
    lane: InLane,
    room: Address,
    policy: kernel::RunPolicy,
}

#[derive(Debug)]
pub(crate) enum Landed {
    Nothing,

    Row { addr: Address, node: NodeId },

    Elsewhere,
}

pub(in crate::worker) struct Flight {
    pool: DrivingPool,
    pub(in crate::worker) gate: RelayGate,
    driving: BTreeMap<RunId, Driving>,

    homes: VecDeque<Arrival>,

    pub(in crate::worker) backlog: runtime::Backlog,

    pub(in crate::worker) edits: runtime::PathLocks,

    pub(in crate::worker) errands: Errands,
}

impl Flight {
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
            errands: Errands::open(gate.bell()),
            gate,
            driving: BTreeMap::new(),
            homes: VecDeque::new(),
            backlog,
            edits: runtime::PathLocks::default(),
        }
    }

    pub(in crate::worker) fn in_flight(&self) -> u32 {
        self.pool.in_flight()
    }

    pub(in crate::worker) fn full(&self) -> bool {
        self.pool.full()
    }

    pub(in crate::worker) fn rows_of(&self, addr: &Address) -> std::collections::BTreeSet<NodeId> {
        self.driving
            .values()
            .filter_map(|driving| match driving.lane.owing.owed() {
                Owed::Row { addr: at, node } if at == addr => Some(node.clone()),
                Owed::Row { .. } | Owed::Asked | Owed::Unasked(_) | Owed::Child { .. } => None,
            })
            .collect()
    }

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

    pub(in crate::worker) fn speaker(&self, run: RunId) -> Option<Speaker<'_>> {
        self.driving.get(&run).map(|driving| Speaker {
            room: &driving.room,
            policy: driving.policy,
            chain: driving.lane.owing.knock_chain(),
        })
    }

    fn issue(&self) -> Relay {
        self.gate.issue()
    }

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

        Some(Ok(Home {
            flown,
            continuation,
            owing,
            open_claims: self.gate.booked.release(run),
        }))
    }
}

struct Home {
    flown: Flown,
    continuation: Continuation,
    owing: Owing,
    open_claims: OpenClaims,
}

impl RunWorker {
    pub(in crate::worker) fn serve_flight(
        &mut self,
        patience: Patience,
    ) -> Result<Landed, AxError> {
        let patience = if self.flight.homes.is_empty() {
            patience
        } else {
            Patience::Now
        };
        let Drained {
            written,
            goals,
            answers,
        } = self
            .flight
            .gate
            .serve(patience, &mut self.ledger, &mut self.flight.homes);
        self.show_relayed(written);
        for ask in goals {
            self.answer_goal(ask);
        }
        for answer in answers {
            self.flight.errands.answered();
            self.take_answer(answer);
        }
        let Some(arrival) = self.flight.arrived() else {
            return Ok(Landed::Nothing);
        };

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

    fn give_back_claims(&mut self, open_claims: OpenClaims) -> Result<(), AxError> {
        let (run, put_backs) = open_claims.owed();
        put_backs
            .into_iter()
            .try_for_each(|line| self.record_for(run, line))
    }

    #[cfg(test)]
    pub(in crate::worker) fn measuring_relay(&self) -> Relay {
        self.flight.issue()
    }

    pub(in crate::worker) fn bell(&self) -> mpsc::Sender<Wake> {
        self.flight.gate.bell()
    }

    pub fn driving(&self) -> bool {
        self.flight.in_flight() > 0
    }

    pub(in crate::worker) fn drives(&self, run: RunId) -> bool {
        self.flight.driving.contains_key(&run)
    }

    pub(in crate::worker) fn land_the_rest(&mut self) -> Result<(), AxError> {
        while self.driving() || self.flight.errands.out() {
            self.serve_flight(Patience::Unbounded)?;
        }
        Ok(())
    }

    pub(in crate::worker) fn stop_the_backlog(&mut self) {
        match self.flight.backlog.halt(None) {
            Ok(0) => {}
            Ok(reached) => self.note(
                runtime::diagnostics::Level::Effect,
                "runtime::backlog",
                &format!("{reached} background commands were stopped as the city closed"),
            ),
            Err(err) => self.note(
                runtime::diagnostics::Level::Refuse,
                "runtime::backlog",
                &format!("the background commands could not be stopped as the city closed: {err}"),
            ),
        }
    }
}
