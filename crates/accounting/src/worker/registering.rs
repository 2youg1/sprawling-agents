// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Goal ground registered on the accounting thread at the moment a
//! model stakes it.
//!
//! Runs dispatched side by side read the same register, so a register a
//! desk copied at dispatch lets both stake the same ground. Each entry is
//! carried here on the accounting thread's one queue instead: the thread
//! arbitrates it against the city's register, writes `goal_registered` or
//! `goal_conflict`, and only then answers, so every later registration
//! reads a register that already holds it (sprawling-SPEC.md 8-42-8).

use std::sync::mpsc;

use kernel::{Address, AxCode, AxError, EventKind, GoalEntry, Payload, RunId};

use super::RunWorker;
use super::relay::Wake;
use accounting::effect;

/// One registration, the run and room its line is filed under, and the
/// address its answer goes back to.
pub(crate) struct GoalAsk {
    run: RunId,
    room: Address,
    entry: GoalEntry,
    back: mpsc::SyncSender<Result<(), AxError>>,
}

/// The booking a run's goal desk registers through: an entry carried on
/// the accounting thread's one queue and waited for, like a relay append.
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
    /// Decides one registration and sends the answer back to its lane. A
    /// lane that stopped listening does not undo the line: the history
    /// holds the outcome either way.
    pub(super) fn answer_goal(&mut self, ask: GoalAsk) {
        let GoalAsk {
            run,
            room,
            entry,
            back,
        } = ask;
        drop(back.send(self.decide_goal(run, room, &entry)));
    }

    /// Arbitrates `entry` against the city's register and writes the
    /// outcome before answering: a registration enters the register
    /// through the folds, and a clash is a fact about the city, not only
    /// a refusal the model saw.
    ///
    /// # Errors
    /// The clash's refusal, a payload that will not build, and the
    /// ledger's refusal of the line.
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
