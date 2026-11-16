// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{Landing, SignalLanded};
use kernel::{AxError, RunId};

use super::super::RunWorker;

#[must_use = "a delivered letter owes its signal_landed line"]
pub(in crate::worker) struct Arrival {
    signal: collab::Signal,

    working: bool,

    knocks: usize,
}

impl Arrival {

    pub(in crate::worker) fn signal(&self) -> &collab::Signal {
        &self.signal
    }
}

impl RunWorker {

    pub(in crate::worker) fn arrive(&mut self, signal: collab::Signal) -> Result<Arrival, AxError> {
        let working = self.collaborating.rooms.worked_by(signal.room()).is_some();
        self.collaborating.rooms.deliver(&signal)?;
        Ok(Arrival {
            signal,
            working,
            knocks: self.doorstep.knocks.len(),
        })
    }

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
