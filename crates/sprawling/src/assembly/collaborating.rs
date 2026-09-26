// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What residents are handing one another: the part of the worker's
//! state that signals, handbacks, pull requests and goal claims change
//! (sprawling-SPEC.md 8-90).

use std::collections::BTreeMap;

use kernel::Address;

use super::RoomQueues;

/// The collaboration the worker holds, one value because one family of
/// records changes it and a desk reads it together.
pub(in crate::assembly) struct Collaborating {
    /// What is waiting for each room, folded from the signal records,
    /// and which run is reading it. A dispatch lends its room's queue
    /// to the signal tool and takes it back when the drive ends, and
    /// `rooms` is what holds that to one queue per room.
    pub(in crate::assembly) rooms: RoomQueues,
    /// What each room already got back from work it handed down. Kept
    /// beside the inboxes because it is folded from the same lines and
    /// belongs to the same room.
    pub(in crate::assembly) joins: BTreeMap<Address, collab::FanIn>,
    /// The graph each room laid out and has not seen join in full, with
    /// what it already handed down. Kept beside `joins` because a
    /// handback landing there is what hands the next ready nodes down.
    /// Held in memory only: after a restart a later run lays the graph
    /// out again and the join skips what already came back.
    pub(in crate::assembly) workshops: BTreeMap<Address, collab::Underway>,
    /// The requests waiting for someone to check them, folded from the
    /// pull request records.
    pub(in crate::assembly) requests: Vec<collab::OpenRequest>,
    /// The ground residents have claimed, folded from `goal_registered`
    /// in the order the claims were made — which is the order the
    /// conflict check reads them in.
    pub(in crate::assembly) goals: Vec<kernel::GoalEntry>,
}
