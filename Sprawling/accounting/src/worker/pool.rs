// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Instant;

use kernel::{AxCode, AxError, RunId};

use super::DriveContext;
use super::dispatching::preparing::{Flown, Staged};
use super::health::Health;
use super::relay::{Relay, Wake};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    pub physical: u64,
    pub available: u64,
}

const RESERVE_SHARE: u64 = 10;

pub(crate) struct Arrival {
    pub run: RunId,
    pub(in crate::worker) flown: Flown,
}

pub(crate) struct DrivingPool {

    home: mpsc::Sender<Wake>,

    running: std::collections::BTreeMap<RunId, std::thread::JoinHandle<()>>,

    trailing: Vec<(RunId, std::thread::JoinHandle<()>)>,

    waiting: VecDeque<Waiting>,

    read_memory: fn() -> Memory,

    monotonic: fn() -> Instant,

    seat_lane: crate::worker::hands::SeatLane,
    health: Health,
}

pub(crate) struct LaneHands {
    pub(crate) read_memory: fn() -> Memory,
    pub(crate) monotonic: fn() -> Instant,
    pub(crate) seat_lane: crate::worker::hands::SeatLane,
}

struct Waiting {
    staged: Staged,
    ledger: Relay,
    context: DriveContext,
    staged_at: Instant,
}

impl DrivingPool {

    pub fn open(home: mpsc::Sender<Wake>, lanes: LaneHands, health: Health) -> DrivingPool {
        DrivingPool {
            home,
            running: std::collections::BTreeMap::new(),
            trailing: Vec::new(),
            waiting: VecDeque::new(),
            read_memory: lanes.read_memory,
            monotonic: lanes.monotonic,
            seat_lane: lanes.seat_lane,
            health,
        }
    }

    pub(crate) fn in_flight(&self) -> u32 {
        u32::try_from(self.running.len()).unwrap_or(u32::MAX)
    }

    pub fn full(&self) -> bool {
        !admits(self.in_flight(), (self.read_memory)())
    }

    pub fn start(
        &mut self,
        staged: Staged,
        ledger: Relay,
        context: DriveContext,
    ) -> Result<(), AxError> {
        let run = staged.run_id();
        if self.running.contains_key(&run)
            || self
                .waiting
                .iter()
                .any(|queued| queued.staged.run_id() == run)
        {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "take a drive into a lane",
                format!("{run} is already driving"),
            )
            .with_recovery("report this: one run drives once"));
        }
        if self.full() {
            self.waiting.push_back(Waiting {
                staged,
                ledger,
                context,
                staged_at: (self.monotonic)(),
            });
            return Ok(());
        }
        self.health.lane_waited(std::time::Duration::ZERO);
        self.open_lane(staged, ledger, context)
    }

    pub(crate) fn start_waiting(&mut self) -> Vec<(RunId, AxError)> {
        let mut refused = Vec::new();

        while !self.full() {
            let Some(Waiting {
                staged,
                ledger,
                context,
                staged_at,
            }) = self.waiting.pop_front()
            else {
                break;
            };
            self.health
                .lane_waited((self.monotonic)().saturating_duration_since(staged_at));
            let run = staged.run_id();
            if let Err(err) = self.open_lane(staged, ledger, context) {
                refused.push((run, err));
            }
        }
        refused
    }

    fn open_lane(
        &mut self,
        staged: Staged,
        mut ledger: Relay,
        context: DriveContext,
    ) -> Result<(), AxError> {
        let run = staged.run_id();
        let home = self.home.clone();
        let seat_lane = self.seat_lane;
        let lane = std::thread::Builder::new()
            .name(format!("sprawling-drive-{run}"))
            .spawn(move || {
                let _seat = seat_lane();
                staged.fly(&mut ledger, context, |flown| {

                    drop(home.send(Wake::Home(Box::new(Arrival { run, flown }))));
                });
            })
            .map_err(|source| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "start a driving lane",
                    source.to_string(),
                )
                .with_recovery("check process thread limits, or drive fewer runs at once")
            })?;
        self.running.insert(run, lane);
        Ok(())
    }

    pub fn landed(&mut self, arrival: Arrival) -> Result<Arrival, AxError> {
        if let Some(lane) = self.running.remove(&arrival.run) {
            self.trailing.push((arrival.run, lane));
        }
        let (finished, going) = std::mem::take(&mut self.trailing)
            .into_iter()
            .partition::<Vec<_>, _>(|(_, lane)| lane.is_finished());
        self.trailing = going;
        finished.into_iter().try_for_each(|(run, lane)| {
            lane.join().map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "close a driving lane",
                    format!("the lane that drove {run} ended abnormally"),
                )
                .with_recovery("restart this city; its history is verified on the way back up")
            })
        })?;
        Ok(arrival)
    }
}

impl Drop for DrivingPool {

    fn drop(&mut self) {
        for (_, lane) in self.trailing.drain(..) {

            drop(lane.join());
        }
    }
}

fn admits(in_flight: u32, memory: Memory) -> bool {
    let tight = memory
        .physical
        .checked_div(RESERVE_SHARE)
        .is_some_and(|reserve| memory.available < reserve);
    in_flight == 0 || !tight
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects)]
mod tests {
    use super::Memory;
    use super::{DrivingPool, admits};
    use kernel::RunId;

    const GIB: u64 = 1 << 30;

    fn tight() -> Memory {
        Memory {
            physical: 16 * GIB,
            available: GIB,
        }
    }

    #[test]
    fn a_pool_judges_memory_by_the_reader_it_was_handed() {
        let (home, _arrivals) = std::sync::mpsc::channel();
        let mut pool = DrivingPool::open(
            home,
            super::LaneHands {
                read_memory: tight,
                monotonic: crate::worker::fixture::monotonic,
                seat_lane: || Box::new(()),
            },
            super::Health::default(),
        );
        pool.running
            .insert(RunId::from_bytes([1u8; 16]), std::thread::spawn(|| {}));

        assert!(pool.full());
    }

    #[test]
    fn a_new_run_waits_while_memory_is_tight() {
        let tight = Memory {
            physical: 16 * GIB,
            available: GIB,
        };
        let roomy = Memory {
            physical: 16 * GIB,
            available: 8 * GIB,
        };
        assert_eq!(
            [
                admits(0, tight),
                admits(1, tight),
                admits(1, roomy),
                admits(64, roomy)
            ],
            [true, false, true, true]
        );
    }
}
