// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a delivered letter landed: a working run's slot, the queue, or
//! a knock for a new run (kernel `spec/Event/Record.lean` D38).
//!
//! Three paths put a letter in a room — a lane's send across the relay,
//! a child's handback, and a settled landing's delivery — and each one
//! takes this decision, so property 8 of `crates/collab/spec/Delivery.lean`
//! (one landing per letter, `knocked` exactly when this delivery queued a
//! knock, `delivered` exactly when a run works in the room) holds for the
//! whole city. The decision reads only the in-process room table and the
//! doorstep, so it is the same on Windows, macOS and Linux.

use kernel::event::record::{Landing, SignalLanded};
use kernel::{AxError, RunId};

use super::super::RunWorker;

/// A letter put in its room, with how the room stood the moment it
/// arrived. It owes one `signal_landed` line, written by
/// [`RunWorker::record_landing`] once the caller has decided whether to
/// knock.
#[must_use = "a delivered letter owes its signal_landed line"]
pub(in crate::worker) struct Arrival {
    signal: collab::Signal,
    /// A run held the room's queue when the letter arrived.
    working: bool,
    /// How many knocks were queued before the caller decided its own.
    knocks: usize,
}

impl Arrival {
    /// The letter as it was delivered.
    pub(in crate::worker) fn signal(&self) -> &collab::Signal {
        &self.signal
    }
}

impl RunWorker {
    /// Puts `signal` in the room it names through the room table, and
    /// remembers whether a run was working there.
    ///
    /// # Errors
    /// Propagates the room table's refusal of the letter.
    pub(in crate::worker) fn arrive(&mut self, signal: collab::Signal) -> Result<Arrival, AxError> {
        let working = self.collaborating.rooms.worked_by(signal.room()).is_some();
        self.collaborating.rooms.deliver(&signal)?;
        Ok(Arrival {
            signal,
            working,
            knocks: self.doorstep.knocks.len(),
        })
    }

    /// Writes where `arrival` landed, under the run and the name its
    /// `signal_enqueued` line was written with, after the caller has
    /// knocked or chosen not to.
    ///
    /// A knock the doorstep already held for the room does not grow the
    /// queue, so the letter reads as queued behind it, as the model's
    /// `City.landing` says.
    ///
    /// # Errors
    /// Propagates a payload that does not build and the ledger's refusal
    /// of the line.
    pub(in crate::worker) fn record_landing(
        &mut self,
        arrival: Arrival,
        run: RunId,
        who: &str,
    ) -> Result<(), AxError> {
        let landing = match (arrival.working, self.doorstep.knocks.len() > arrival.knocks) {
            (true, _) => Landing::Delivered,
            (false, true) => Landing::Knocked,
            (false, false) => Landing::Queued,
        };
        let data = kernel::Payload::of(&SignalLanded {
            signal: arrival.signal.id().clone(),
            landing,
        })?;
        self.record_for(
            run,
            crate::effect::Line {
                who: who.to_owned(),
                addr: arrival.signal.room().clone(),
                kind: kernel::EventKind::SignalLanded,
                data,
            },
        )
    }
}
