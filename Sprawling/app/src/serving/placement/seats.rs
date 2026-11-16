// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The plan's seats in the model's two pools
//! (`crates/sprawling/spec/Serving/Placement.lean`, the two-pool section):
//! the plan's last [`SERIAL_SEATS`] seats belong to the city's serial
//! threads, the rest to the runs' lanes, so a lane cannot take the seat a
//! serial thread needs and no seat holds two threads. This part is the
//! table alone, with no platform call and no setting: the module above
//! builds it and hands a seat to the platform.
//!
//! `crates/sprawling/src/serving/placement/tests.rs` checks these
//! properties on the table over every plan of up to five seats and every
//! sequence of six starts and exits of four threads.

use super::plan::Processor;

/// Who holds a seat: one per hot thread for as long as it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Holder(pub(crate) u64);

/// Which kind of hot thread wants a seat. A serial thread is one of the
/// city's one-per-city threads, which every relay and every broadcast
/// waits for; a lane is one run's own thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Serial,
    Lane,
}

/// One seat: where it is, and who sits there now.
#[derive(Debug)]
struct Sitting {
    seat: Processor,
    held_by: Option<Holder>,
}

/// The seats of one plan in the model's two pools (`Seats` and `step`,
/// one pool per role): the plan's last [`SERIAL_SEATS`] processors are
/// the serial threads' pool, the rest are the lanes'. A lane cannot take
/// a seat a serial thread needs, and no seat holds two threads, so the
/// seated threads never outnumber the plan.
#[derive(Debug)]
pub(crate) struct Seats {
    serial: Vec<Sitting>,
    lanes: Vec<Sitting>,
}

impl Seats {
    /// The plan's seats in the two pools: lanes take the plan's
    /// processors from the front, serial threads its last
    /// [`SERIAL_SEATS`]. A plan with fewer seats than that is all serial
    /// pool, because those are the threads every other thread waits for.
    pub(crate) fn new(plan: Vec<Processor>) -> Self {
        let mut lanes = plan;
        let serial = lanes.split_off(lanes.len().saturating_sub(SERIAL_SEATS));
        let free = |seat: Processor| Sitting {
            seat,
            held_by: None,
        };
        Self {
            serial: serial.into_iter().map(free).collect(),
            lanes: lanes.into_iter().map(free).collect(),
        }
    }

    fn pool_mut(&mut self, role: Role) -> &mut [Sitting] {
        match role {
            Role::Serial => &mut self.serial,
            Role::Lane => &mut self.lanes,
        }
    }

    /// Seats `holder` on the first free seat of its own pool. A holder
    /// already seated keeps its seat; with no free seat in its pool the
    /// holder gets none, and the scheduler places it.
    pub(crate) fn start(&mut self, holder: Holder, role: Role) {
        if self.seat_of(holder).is_some() {
            return;
        }
        if let Some(free) = self
            .pool_mut(role)
            .iter_mut()
            .find(|sitting| sitting.held_by.is_none())
        {
            free.held_by = Some(holder);
        }
    }

    /// Gives `holder`'s seat back; no other seat moves.
    pub(crate) fn exit(&mut self, holder: Holder) {
        for pool in [&mut self.serial, &mut self.lanes] {
            for sitting in pool {
                if sitting.held_by == Some(holder) {
                    sitting.held_by = None;
                }
            }
        }
    }

    /// The processor `holder` sits at.
    pub(crate) fn seat_of(&self, holder: Holder) -> Option<Processor> {
        [&self.serial, &self.lanes]
            .into_iter()
            .flatten()
            .find(|sitting| sitting.held_by == Some(holder))
            .map(|sitting| sitting.seat)
    }
}

/// How many seats of a plan belong to the serial threads: the ledger
/// thread and the view fold, the city's two one-per-city hot threads.
pub(crate) const SERIAL_SEATS: usize = 2;
