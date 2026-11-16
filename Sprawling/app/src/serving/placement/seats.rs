// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::plan::Processor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Holder(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Serial,
    Lane,
}

#[derive(Debug)]
struct Sitting {
    seat: Processor,
    held_by: Option<Holder>,
}

#[derive(Debug)]
pub(crate) struct Seats {
    serial: Vec<Sitting>,
    lanes: Vec<Sitting>,
}

impl Seats {

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

    pub(crate) fn exit(&mut self, holder: Holder) {
        for pool in [&mut self.serial, &mut self.lanes] {
            for sitting in pool {
                if sitting.held_by == Some(holder) {
                    sitting.held_by = None;
                }
            }
        }
    }

    pub(crate) fn seat_of(&self, holder: Holder) -> Option<Processor> {
        [&self.serial, &self.lanes]
            .into_iter()
            .flatten()
            .find(|sitting| sitting.held_by == Some(holder))
            .map(|sitting| sitting.seat)
    }
}

pub(crate) const SERIAL_SEATS: usize = 2;
