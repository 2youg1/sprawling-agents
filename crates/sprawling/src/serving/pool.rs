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

use std::sync::mpsc;
use std::time::Duration;

use kernel::{AxCode, AxError, RunId};

use super::relay::Relay;
use crate::assembly::{DriveContext, Driven, Driving, drive_run};

/// How many runs a city drives at once.
///
/// It equals `gateway::admission`'s per-provider ceiling on purpose,
/// and it is deliberately *not* read from there: that ceiling is
/// `pub(crate)` inside `gateway`, and publishing it is a change to that
/// crate's public surface with a baseline of its own to recompute
/// (sprawling-SPEC.md 8-46-3, 8-46-8). Until that card lands, a wider
/// pool would only park lanes in admission — which moves the queue from
/// somewhere that can count to somewhere that cannot.
pub(crate) const DRIVING_LANES: u32 = 4;

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
    /// Handed to each lane, kept here so the receiver below stays
    /// connected while the pool lives.
    sending: mpsc::Sender<Arrival>,
    arriving: mpsc::Receiver<Arrival>,
    /// One entry per run still driving. The handle is joined where the
    /// run comes home, so a pool that reports nothing in flight has no
    /// thread left behind it.
    running: std::collections::BTreeMap<RunId, std::thread::JoinHandle<()>>,
}

impl DrivingPool {
    pub(crate) fn open(lanes: u32) -> DrivingPool {
        let (sending, arriving) = mpsc::channel();
        DrivingPool {
            lanes: lanes.max(1),
            sending,
            arriving,
            running: std::collections::BTreeMap::new(),
        }
    }

    /// How many runs are driving right now. The `in_flight` half of the
    /// stop condition in `kernel::pursuit`.
    pub(crate) fn in_flight(&self) -> u32 {
        u32::try_from(self.running.len()).unwrap_or(u32::MAX)
    }

    pub(crate) fn full(&self) -> bool {
        self.in_flight() >= self.lanes
    }

    /// Takes one drive into a lane of its own.
    ///
    /// The relay is this run's write face and is moved in with it: a
    /// lane that could be handed a second one could write for a run it
    /// is not driving.
    ///
    /// # Errors
    /// Refuses when the operating system will not start a thread, and
    /// when this run is already driving — two runs under one id would
    /// make the pool's own table lie about what is in flight.
    pub(crate) fn start(
        &mut self,
        driving: Driving,
        mut ledger: Relay,
        context: DriveContext,
    ) -> Result<(), AxError> {
        let run = driving.run_id;
        if self.running.contains_key(&run) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "take a drive into a lane",
                format!("{run} is already driving"),
            )
            .with_recovery("report this: one run drives once"));
        }
        let home = self.sending.clone();
        let lane = std::thread::Builder::new()
            .name(format!("sprawling-drive-{run}"))
            .spawn(move || {
                let driven = drive_run(driving, &mut ledger, context);
                // Nobody listening means the city stopped pursuing while
                // this run was going: the history already has whatever
                // this drive wrote, and there is nothing left to tell.
                let _ = home.send(Arrival { run, driven });
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

    /// The next run home, or `None` if none arrived inside `wait`.
    ///
    /// `None` is not "nothing is running": the caller waits in short
    /// steps so that it can serve the relay in between, which is what
    /// lets a lane make progress at all.
    ///
    /// # Errors
    /// Refuses when a lane ended without saying so, which under
    /// `panic = "abort"` cannot happen in a shipped binary and can in a
    /// test build that unwinds.
    pub(crate) fn arrived(&mut self, wait: Duration) -> Option<Result<Arrival, AxError>> {
        let arrival = self.arriving.recv_timeout(wait).ok()?;
        let joined = match self.running.remove(&arrival.run) {
            Some(lane) => lane.join(),
            None => return Some(Ok(arrival)),
        };
        if joined.is_err() {
            return Some(Err(AxError::failure(
                AxCode::StorageFatal,
                "close a driving lane",
                format!("the lane driving {} ended abnormally", arrival.run),
            )
            .with_recovery(
                "restart this city; its history is verified on the way back up",
            )));
        }
        Some(Ok(arrival))
    }
}
