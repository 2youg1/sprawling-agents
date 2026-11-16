// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::mpsc;

use kernel::{Address, AxCode, AxError, EventKind, GoalEntry, Payload, RunId};

use super::RunWorker;
use super::relay::Wake;
use crate::effect;

pub(crate) struct GoalAsk {
    run: RunId,
    room: Address,
    entry: GoalEntry,
    back: mpsc::SyncSender<Result<(), AxError>>,
}

pub(crate) fn booking(bell: mpsc::Sender<Wake>, run: RunId, room: Address) -> collab::GoalBooking {
    collab::GoalBooking::new(move |entry: &GoalEntry| {
        let (back, answer) = mpsc::sync_channel(0);
        bell.send(Wake::Goal(GoalAsk {
            run,
            room: room.clone(),
            entry: entry.clone(),
            back,
        }))
        .map_err(|_| gone("the accounting thread is no longer taking registrations"))?;
        answer
            .recv()
            .map_err(|_| gone("the accounting thread ended before answering"))?
    })
}

impl RunWorker {
    pub(super) fn answer_goal(&mut self, ask: GoalAsk) {
        let GoalAsk {
            run,
            room,
            entry,
            back,
        } = ask;
        drop(back.send(self.decide_goal(run, room, &entry)));
    }

    fn decide_goal(&mut self, run: RunId, room: Address, entry: &GoalEntry) -> Result<(), AxError> {
        let clash = collab::arbitrate(&self.collaborating.goals, entry);
        let (kind, data) = match &clash {
            None => (EventKind::GoalRegistered, Payload::of(entry)?),
            Some(level) => (
                EventKind::GoalConflict,
                collab::conflict_payload(entry, level)?,
            ),
        };
        self.record_for(
            run,
            effect::Line {
                who: entry.owner.clone(),
                addr: room,
                kind,
                data,
            },
        )?;
        clash.map_or(Ok(()), |level| Err(collab::conflict_refusal(entry, &level)))
    }
}

fn gone(why: &str) -> AxError {
    AxError::failure(AxCode::StorageFatal, "register a goal", why)
        .with_recovery("the city is closing; resume it and register the goal again")
}
