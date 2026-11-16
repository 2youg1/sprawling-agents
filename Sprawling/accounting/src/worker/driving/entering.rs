// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, NodeId, RunId};

use super::super::{Continuation, Owing, RunWorker, Unasked};
use super::flight::InLane;
use crate::worker::dispatching::preparing::Staged;

impl RunWorker {

    pub(in crate::worker) fn dispatch_into_lane(
        &mut self,
        at: super::super::Assignment,
        task: String,
        goal: String,
        owing: Owing,
    ) -> Result<RunId, AxError> {
        let (staged, continuation) = self.stage_dispatch(at, task, goal)?;
        self.enter_lane(staged, continuation, owing)
    }

    pub(in crate::worker) fn start_unasked(
        &mut self,
        addr: Address,
        task: String,
        goal: String,
        because: Unasked,
    ) -> Option<RunId> {
        let at = super::super::Assignment {
            addr: addr.clone(),

            origin: None,
            session: None,
            effort: None,
            difficulty: None,
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

    pub(in crate::worker) fn take_row_into_lane(
        &mut self,
        staged: Staged,
        addr: Address,
        node: NodeId,
        continuation: Continuation,
    ) -> Result<RunId, AxError> {
        self.enter_lane(staged, continuation, Owing::row(addr, node))
    }

    fn enter_lane(
        &mut self,
        staged: Staged,
        continuation: Continuation,
        owing: Owing,
    ) -> Result<RunId, AxError> {
        let run = staged.run_id();
        if let Some(handing) = continuation.handing(staged.assignment(), &owing) {
            self.collaborating.handing.insert(run, handing);
        }
        let context = self.drive_context();
        let taken = self.flight.take(
            staged,
            context,
            InLane {
                continuation,
                owing,
            },
        );
        if taken.is_err() {
            self.collaborating.handing.remove(&run);
        }
        taken
    }
}
