// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The seat each hot thread takes on the ranked processor table: the
//! least-occupied core, the better one on a tie, kept until the thread
//! exits, and handed to the platform as a soft ideal processor so the
//! scheduler moves the thread on at once when that core is busy
//! (`crates/sprawling/spec/Serving/Placement.lean`, decision D41). The
//! two properties every start and exit sequence keeps are proved there;
//! `tests` checks this table against them.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// Which placement the hot threads get. A comparison sitting changes
/// this one value; hard affinity is applied to the whole process from
/// outside (D41), so it is not an arm of the seat table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    /// No seat and no platform call: the scheduler alone decides.
    #[expect(dead_code, reason = "the comparison arm a measuring sitting turns on")]
    Off,
    /// A seat and a soft ideal processor.
    Soft,
}

const ARM: Arm = Arm::Soft;

/// One logical processor: its rank, smaller is better, and the number
/// the platform knows it by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Core {
    pub(crate) rank: u8,
    pub(crate) processor: u32,
}

/// The processor table, sorted best first; the sort is in the
/// constructor, so a table is ranked by construction (`RankedBest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cores(Vec<Core>);

impl Cores {
    pub(crate) fn ranked(mut cores: Vec<Core>) -> Self {
        cores.sort_by_key(|core| core.rank);
        Self(cores)
    }

    pub(crate) fn get(&self, index: usize) -> Option<Core> {
        self.0.get(index).copied()
    }
}

/// Who holds a seat: one per hot thread for as long as it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Holder(pub(crate) u64);

/// The seats on one processor table (the model's `Seats` and `step`).
#[derive(Debug)]
pub(crate) struct SeatTable {
    cores: Cores,
    load: Vec<u32>,
    seats: BTreeMap<Holder, usize>,
}

impl SeatTable {
    pub(crate) fn new(cores: Cores) -> Self {
        let load = vec![0; cores.0.len()];
        Self {
            cores,
            load,
            seats: BTreeMap::new(),
        }
    }

    /// Seats `holder` on the least-occupied core, the better on a tie. A
    /// holder already seated keeps its seat; an empty table seats nobody.
    pub(crate) fn start(&mut self, holder: Holder) {
        if self.seats.contains_key(&holder) {
            return;
        }
        let next = self.seats.len();
        let Some(index) = next.checked_rem(self.load.len()) else {
            return;
        };
        if let Some(load) = self.load.get_mut(index) {
            *load = load.saturating_add(1);
            self.seats.insert(holder, index);
        }
    }

    /// Gives `holder`'s seat back; no other seat moves.
    pub(crate) fn exit(&mut self, holder: Holder) {
        if let Some(index) = self.seats.remove(&holder)
            && let Some(load) = self.load.get_mut(index)
        {
            *load = load.saturating_sub(1);
        }
    }

    /// The index in the processor table `holder` sits at.
    pub(crate) fn seat_of(&self, holder: Holder) -> Option<usize> {
        self.seats.get(&holder).copied()
    }

    pub(crate) fn core(&self, index: usize) -> Option<Core> {
        self.cores.get(index)
    }
}

/// The one table every hot thread of this process sits at, built from
/// the platform's ranks at the first seat.
static TABLE: Mutex<Option<SeatTable>> = Mutex::new(None);
static NEXT_HOLDER: AtomicU64 = AtomicU64::new(0);

/// A hot thread's seat; dropping it, which a thread does as it exits,
/// gives the seat back.
#[derive(Debug)]
pub(crate) struct Seat(Option<Holder>);

impl Seat {
    /// A seat that holds nothing, for a thread built in a test.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self(None)
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        if let Some(holder) = self.0 {
            let mut table = TABLE.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(table) = table.as_mut() {
                table.exit(holder);
            }
        }
    }
}

/// Takes a seat for the calling thread and makes its core the thread's
/// soft ideal processor. A refusal is told on stderr once, here, and the
/// thread runs where the scheduler puts it.
pub(crate) fn seat_this_thread(name: &'static str) -> Seat {
    match ARM {
        Arm::Off => Seat(None),
        Arm::Soft => {
            let holder = Holder(NEXT_HOLDER.fetch_add(1, Ordering::Relaxed));
            let core = {
                let mut table = TABLE.lock().unwrap_or_else(PoisonError::into_inner);
                let table = table.get_or_insert_with(|| SeatTable::new(platform_cores()));
                table.start(holder);
                table.seat_of(holder).and_then(|index| table.core(index))
            };
            if let Some(core) = core
                && let Err(reason) = prefer(core.processor)
            {
                eprintln!(
                    "thread {name} runs without an ideal processor (processor {}): {reason}",
                    core.processor
                );
            }
            Seat(Some(holder))
        }
    }
}

/// Windows: one rank over the processors of group 0, the fallback D41
/// names until the efficiency classes are read; the table then only
/// spreads the hot threads, least-occupied first.
#[cfg(windows)]
fn platform_cores() -> Cores {
    /// An ideal processor is an index inside the thread's processor group,
    /// and a group holds at most 64.
    const GROUP: u32 = 64;
    let count = match std::thread::available_parallelism() {
        Ok(count) => count.get(),
        Err(err) => {
            eprintln!(
                "the hot threads get no ideal processor: the processor count is unreadable: {err}"
            );
            0
        }
    };
    Cores::ranked(
        (0..GROUP)
            .take(count)
            .map(|processor| Core { rank: 0, processor })
            .collect(),
    )
}

/// macOS and Linux have no soft ideal processor, so the table is empty
/// and seats nobody (D41).
#[cfg(not(windows))]
fn platform_cores() -> Cores {
    Cores::ranked(Vec::new())
}

#[cfg(windows)]
fn prefer(processor: u32) -> Result<(), thread_priority::Error> {
    thread_priority::windows::set_current_thread_ideal_processor(processor).map(|_previous| ())
}

#[cfg(not(windows))]
fn prefer(_processor: u32) -> Result<(), std::convert::Infallible> {
    Ok(())
}

#[cfg(test)]
mod tests;
