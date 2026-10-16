// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The provider requests the accounting thread hands off: each one on a
//! thread of its own, its answer brought back on the one queue and
//! recorded there (`crates/sprawling/Spec.lean` §8-42-9).
//!
//! **The accounting thread never waits on a provider.** It is the
//! city's one writer, so every relay request, landing and command waits
//! for as long as it does (ARCHITECTURE §13.4). What to ask stays on
//! that thread, and so does the record of the answer; only the network
//! leaves.
//!
//! One thread per request rather than the driving pool or one resident
//! thread, so a renewal of one door never waits for another door or for
//! a probe (`crates/sprawling/Spec.lean` D78).

use std::sync::mpsc;

use kernel::{AxCode, AxError};

use super::RunWorker;
use super::credentials::endpoints::{Attaching, Probed};
use super::keeping_warm::Renewed;
use super::relay::Wake;

/// What a handed-off request came back with, one arm per kind of
/// request, each recorded by the module that owns that kind.
pub(crate) enum Answer {
    /// A keep-warm door renewed its due prefixes, or was refused.
    Renewed(Renewed),
    /// A `ProbeEndpoint` read the endpoint.
    Probed(Probed),
    /// An `AttachEndpoint` asked the endpoint what it serves.
    Attached(Attaching),
}

/// The requests out on threads of their own, and the queue their
/// answers come back on.
pub(in crate::worker) struct Errands {
    bell: mpsc::Sender<Wake>,
    /// Sent and not yet taken off the queue. Counted on the accounting
    /// thread when the answer is taken, so a close that waits while
    /// this is above zero cannot stop between an answer queued and its
    /// record written.
    out: u32,
}

impl Errands {
    pub(in crate::worker) fn open(bell: mpsc::Sender<Wake>) -> Errands {
        Errands { bell, out: 0 }
    }

    /// Runs `request` on a thread of its own and puts what it returns on
    /// the accounting thread's queue as [`Wake::Answered`]. The thread
    /// ends there.
    ///
    /// # Errors
    /// Refuses when the operating system will not start a thread; the
    /// request is dropped unsent.
    pub(in crate::worker) fn send(
        &mut self,
        request: impl FnOnce() -> Answer + Send + 'static,
    ) -> Result<(), AxError> {
        let bell = self.bell.clone();
        std::thread::Builder::new()
            .name("sprawling-errand".to_owned())
            .spawn(move || {
                // Nobody listening means the city stopped attending; the
                // answer has nowhere left to be recorded.
                drop(bell.send(Wake::Answered(Box::new(request()))));
            })
            .map_err(|source| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "send a provider request off the accounting thread",
                    source.to_string(),
                )
                .with_recovery("check process thread limits, then ask again")
            })?;
        self.out = self.out.saturating_add(1);
        Ok(())
    }

    /// One answer was taken off the queue.
    pub(in crate::worker) fn answered(&mut self) {
        self.out = self.out.saturating_sub(1);
    }

    /// Whether a request is still out.
    pub(in crate::worker) fn out(&self) -> bool {
        self.out > 0
    }
}

impl RunWorker {
    /// Records one answer through the module that owns its kind of
    /// request.
    pub(in crate::worker) fn take_answer(&mut self, answer: Answer) {
        match answer {
            Answer::Renewed(renewed) => self.record_renewal(renewed),
            Answer::Probed(probed) => self.record_probe(probed),
            Answer::Attached(attaching) => self.finish_attach(attaching),
        }
    }
}
