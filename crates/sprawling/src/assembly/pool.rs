// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The driving lanes: one thread per run in flight, entered with a
//! drive and left with what that drive left behind.
//!
//! A lane holds one thing, a drive, and the only `kernel::Ledger` it is
//! given is a [`Relay`](super::relay::Relay) — so "a city has one
//! writer" is held by the types rather than by discipline: a lane
//! cannot reach the segments at all (sprawling-SPEC.md 8-46-3).
//!
//! **A lane is a thread that lives exactly as long as the run it
//! drives.** A resident pool of threads would need an entrance channel
//! and a closing protocol to hold nothing between runs; here the
//! `JoinHandle` *is* the evidence that a run is still going, and its
//! end is the run's end.

use std::collections::VecDeque;
use std::sync::mpsc;

use kernel::{AxCode, AxError, RunId};

use super::relay::{Relay, Wake};
use super::{DriveContext, Driven, Driving, drive_run};
use crate::monitor::memory::Memory;

/// How many runs a city drives at once, whichever entrance started
/// them: past it, a prepared drive waits in the pool for a lane.
///
/// It equals `gateway::admission`'s per-provider ceiling on purpose,
/// and it is deliberately *not* read from there: that ceiling is
/// `pub(crate)` inside `gateway`, and publishing it is a change to that
/// crate's public surface with a baseline of its own to recompute
/// (sprawling-SPEC.md 8-46-3, 8-46-8). Until that card lands, a wider
/// pool would only park lanes in admission — which moves the queue from
/// somewhere that can count to somewhere that cannot.
pub(crate) const DRIVING_LANES: u32 = 4;

/// The share of physical memory a new run leaves free: one part in
/// this many. A share rather than a byte count, because a byte count
/// suits one class of machine (sprawling-SPEC.md 8-46-3).
const RESERVE_SHARE: u64 = 10;

/// One run, home from its lane: which run it was, and what its drive
/// left behind. The two travel together because neither is anything
/// without the other — a `Driven` names no run, and a run id says
/// nothing about what to settle.
pub(crate) struct Arrival {
    pub(crate) run: RunId,
    pub(crate) driven: Result<Driven, AxError>,
}

/// The lanes a city drives in.
///
/// Nothing here decides anything: which runs to start and what to do
/// with one that came home are the caller's, and both of those are
/// judgements about a plan rather than about a thread.
pub(crate) struct DrivingPool {
    lanes: u32,
    /// Handed to each lane: a run comes home on the accounting
    /// thread's one queue, beside the relay requests it wrote.
    home: mpsc::Sender<Wake>,
    /// One entry per run still driving. The handle is joined where the
    /// run comes home, so a pool that reports nothing in flight has no
    /// thread left behind it.
    running: std::collections::BTreeMap<RunId, std::thread::JoinHandle<()>>,
    /// Drives prepared while every lane was taken, oldest first. A
    /// waiting drive holds no thread: the queue is here, where it can be
    /// counted, rather than in threads parked on the provider's
    /// admission.
    waiting: VecDeque<Waiting>,
}

/// One prepared drive and the two things its lane will be given.
struct Waiting {
    driving: Driving,
    ledger: Relay,
    context: DriveContext,
}

impl DrivingPool {
    pub(crate) fn open(lanes: u32, home: mpsc::Sender<Wake>) -> DrivingPool {
        DrivingPool {
            lanes: lanes.max(1),
            home,
            running: std::collections::BTreeMap::new(),
            waiting: VecDeque::new(),
        }
    }

    /// How many runs are driving right now. The `in_flight` half of the
    /// stop condition in `kernel::pursuit`.
    ///
    /// A drive in the waiting queue is not counted, although it has
    /// already written its `run_started` line. The drain check stays
    /// sound only because a drive waits solely while [`Self::full`]
    /// holds, `full` never holds with no run driving, and
    /// [`Self::start_waiting`] runs whenever a lane comes home: so this
    /// reads zero only when nothing waits either. A change that lets a
    /// drive wait for another reason counts the waiting drives here.
    pub(crate) fn in_flight(&self) -> u32 {
        u32::try_from(self.running.len()).unwrap_or(u32::MAX)
    }

    /// Whether a new run waits: every lane is taken, or memory is
    /// tight while another run is driving (sprawling-SPEC.md 8-46-3).
    pub(crate) fn full(&self, memory: Memory) -> bool {
        !admits(self.in_flight(), self.lanes, memory)
    }

    /// Takes one drive into a lane of its own, or into the queue when
    /// every lane is taken.
    ///
    /// The relay is this run's write face and is moved in with it: a
    /// lane that could be handed a second one could write for a run it
    /// is not driving.
    ///
    /// # Errors
    /// Refuses when the operating system will not start a thread, and
    /// when this run is already driving or waiting — two runs under one
    /// id would make the pool's own table lie about what is in flight.
    pub(crate) fn start(
        &mut self,
        driving: Driving,
        ledger: Relay,
        context: DriveContext,
    ) -> Result<(), AxError> {
        let run = driving.run_id;
        if self.running.contains_key(&run)
            || self
                .waiting
                .iter()
                .any(|queued| queued.driving.run_id == run)
        {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "take a drive into a lane",
                format!("{run} is already driving"),
            )
            .with_recovery("report this: one run drives once"));
        }
        if self.full(crate::monitor::memory::read()) {
            self.waiting.push_back(Waiting {
                driving,
                ledger,
                context,
            });
            return Ok(());
        }
        self.open_lane(driving, ledger, context)
    }

    /// Starts the drives that waited for a lane, oldest first, until the
    /// lanes are full again. Called once a lane has come home, and
    /// before the run it carried is landed, so work that landing sends
    /// out queues behind work that was already waiting.
    ///
    /// Returns each run whose lane would not start, with the refusal.
    pub(crate) fn start_waiting(&mut self) -> Vec<(RunId, AxError)> {
        let mut refused = Vec::new();
        // Memory is read at each start, because every lane started here
        // is a run the next reading has to make room for.
        while !self.full(crate::monitor::memory::read()) {
            let Some(Waiting {
                driving,
                ledger,
                context,
            }) = self.waiting.pop_front()
            else {
                break;
            };
            let run = driving.run_id;
            if let Err(err) = self.open_lane(driving, ledger, context) {
                refused.push((run, err));
            }
        }
        refused
    }

    /// Opens the thread one drive runs on.
    ///
    /// # Errors
    /// Refuses when the operating system will not start a thread.
    fn open_lane(
        &mut self,
        driving: Driving,
        mut ledger: Relay,
        context: DriveContext,
    ) -> Result<(), AxError> {
        let run = driving.run_id;
        let home = self.home.clone();
        let lane = std::thread::Builder::new()
            .name(format!("sprawling-drive-{run}"))
            .spawn(move || {
                let driven = drive_run(driving, &mut ledger, context);
                // Nobody listening means the city stopped pursuing while
                // this run was going: the history already has whatever
                // this drive wrote, and there is nothing left to tell.
                drop(home.send(Wake::Home(Box::new(Arrival { run, driven }))));
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

    /// Closes the lane `arrival` came home from.
    ///
    /// # Errors
    /// Refuses when a lane ended without saying so, which under
    /// `panic = "abort"` cannot happen in a shipped binary and can in a
    /// test build that unwinds.
    pub(crate) fn landed(&mut self, arrival: Arrival) -> Result<Arrival, AxError> {
        let Some(lane) = self.running.remove(&arrival.run) else {
            return Ok(arrival);
        };
        if lane.join().is_err() {
            return Err(AxError::failure(
                AxCode::StorageFatal,
                "close a driving lane",
                format!("the lane driving {} ended abnormally", arrival.run),
            )
            .with_recovery("restart this city; its history is verified on the way back up"));
        }
        Ok(arrival)
    }
}

/// Whether one more run may start: a lane is free, and either no run is
/// driving or the machine keeps a tenth of its physical memory free.
fn admits(in_flight: u32, lanes: u32, memory: Memory) -> bool {
    let tight = memory
        .physical
        .checked_div(RESERVE_SHARE)
        .is_some_and(|reserve| memory.available < reserve);
    in_flight < lanes && (in_flight == 0 || !tight)
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects)]
mod tests {
    use super::admits;
    use crate::monitor::memory::Memory;

    const GIB: u64 = 1 << 30;

    /// A machine with a sixteenth of its memory left starts no second
    /// run beside the first, and still starts the first, so a city on a
    /// machine that stays tight keeps moving.
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
                admits(0, 4, tight),
                admits(1, 4, tight),
                admits(1, 4, roomy),
                admits(4, 4, roomy)
            ],
            [true, false, true, false]
        );
    }
}
