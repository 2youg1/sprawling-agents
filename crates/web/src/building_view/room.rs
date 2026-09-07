// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The room: waiting signals in one room.

use channels::{Address, InboxAnswer};

use super::leaf::room_addr;

/// What this page knows about one room's queue.
///
/// `Unasked` and `Empty` are different answers and must stay different:
/// an answer that belongs to another room, or has not arrived, would
/// otherwise be rendered as "nothing waits here" - which is a claim this
/// page has no basis for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomQueue {
    Unasked,
    Empty,
    Waiting(Vec<channels::SignalLine>),
}

/// Reads the held answer as this room's queue.
///
/// Looking is not taking: the city folds the queue from the Ledger, so a
/// view of it consumes nothing. That is why this page can show a mailbox
/// at all - `Inbox::pull` would empty what it reported.
#[must_use]
pub fn waiting_in(inbox: Option<&InboxAnswer>, building: &Address, room: &str) -> RoomQueue {
    let Some(held) = inbox else {
        return RoomQueue::Unasked;
    };
    if room_addr(building, room).is_none_or(|at| held.addr != at) {
        return RoomQueue::Unasked;
    }
    if held.waiting.is_empty() {
        RoomQueue::Empty
    } else {
        RoomQueue::Waiting(held.waiting.clone())
    }
}

/// A day count rendered the way the archive files it: whole days, because
/// a stamp with more precision than the question invites comparisons
/// nobody meant to make.
#[must_use]
pub fn day_label(day: u64) -> String {
    format!("day {day}")
}
