// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::Address;

use super::RoomQueues;

pub(in crate::worker) struct Collaborating {
    pub(in crate::worker) rooms: RoomQueues,

    pub(in crate::worker) joins: BTreeMap<Address, collab::FanIn>,

    pub(in crate::worker) workshops: BTreeMap<Address, collab::Underway>,

    pub(in crate::worker) handing: BTreeMap<kernel::RunId, super::waking::handing::Handing>,

    pub(in crate::worker) requests: Vec<collab::OpenRequest>,

    pub(in crate::worker) goals: Vec<kernel::GoalEntry>,
}

pub(in crate::worker) fn register_goal(
    goals: &mut Vec<kernel::GoalEntry>,
    kind: kernel::EventKind,
    data: &kernel::Payload,
) -> Result<(), kernel::AxError> {
    if kind == kernel::EventKind::GoalRegistered {
        goals.push(data.read()?);
    }
    Ok(())
}
