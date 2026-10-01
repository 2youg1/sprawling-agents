// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three doors a run enters a lane through: a person's command, a
//! run the city asks for itself, and a row a pursuit takes. Each carries
//! into the lane what the city will owe the run when it comes home.

use kernel::{Address, AxError, NodeId, RunId};

use super::super::{Continuation, Owing, RunWorker, Unasked};
use super::flight::InLane;
use crate::worker::dispatching::preparing::Staged;

impl RunWorker {
    /// Stages one dispatch on this thread and takes it into a lane, which
    /// prepares and drives it, so the desk is free again before a tree is
    /// placed or a server has shaken hands (sprawling-SPEC.md 8-113).
    ///
    /// This is the person's entrance. What continues is the run, not the
    /// command, which is why the idempotency key settles at take-off
    /// (sprawling-SPEC.md 8-46-2).
    ///
    /// # Errors
    /// Propagates every refusal a dispatch can owe before it costs
    /// anything, and the pool's refusal to start a lane.
    pub(in crate::worker) fn dispatch_into_lane(
        &mut self,
        at: super::super::Assignment,
        task: String,
        goal: String,
        owing: Owing,
    ) -> Result<RunId, AxError> {
        let (staged, continuation) = self.stage_dispatch(at, task, goal)?;
        let context = self.drive_context();
        self.flight.take(
            staged,
            context,
            InLane {
                continuation,
                owing,
            },
        )
    }

    /// Starts a run the city asked for itself, and says why in the log
    /// when it cannot be started.
    ///
    /// The three entrances nobody types a command at take this door:
    /// the schedule, an arrival from outside, and work a person just
    /// unblocked. None of them has a person waiting on the answer, so a
    /// refusal is noted against the reason the run existed rather than
    /// failing whatever was running at the time.
    pub(in crate::worker) fn start_unasked(
        &mut self,
        addr: Address,
        task: String,
        goal: String,
        because: Unasked,
    ) -> Option<RunId> {
        let at = super::super::Assignment {
            addr: addr.clone(),
            // A row in a plan is not a session's first run: whatever the
            // room branched from was spent by the run that began it.
            origin: None,
            session: None,
            effort: None,
            model: None,
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            parent: None,
            succession: None,
            taint: because.taint(),
            dispatched_by: kernel::event::Who::City,
        };
        let reason = because.because();
        match self.dispatch_into_lane(at, task, goal, Owing::unasked(because)) {
            Ok(run) => Some(run),
            Err(err) => {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "accounting::worker",
                    &format!(
                        "no run started at {} although {}: {}",
                        addr.as_str(),
                        reason,
                        err.subject()
                    ),
                );
                None
            }
        }
    }

    /// Puts a staged dispatch into a lane on a pursuit's behalf.
    ///
    /// # Errors
    /// Propagates the pool's refusal to start a lane.
    pub(in crate::worker) fn take_row_into_lane(
        &mut self,
        staged: Staged,
        addr: Address,
        node: NodeId,
        continuation: Continuation,
    ) -> Result<RunId, AxError> {
        let context = self.drive_context();
        self.flight.take(
            staged,
            context,
            InLane {
                continuation,
                owing: Owing::row(addr, node),
            },
        )
    }
}
