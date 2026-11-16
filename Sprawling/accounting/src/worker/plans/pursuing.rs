// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, NodeId};

use super::super::{Assignment, Landed, RunWorker};

impl RunWorker {
    pub(in crate::worker) fn advance_pursuits(&mut self, landed: &Landed) {
        if let Landed::Row { addr, node } = landed
            && self.ready_in(addr).contains(node)
        {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "kernel::pursuit",
                &format!(
                    "{node} is still ready after a run took it; the pursuit pauses rather than \
                     dispatching it again"
                ),
            );
            if let Err(err) = self.set_pursuit(addr, wire::PursuitStep::Pause) {
                self.pursuit_refused(addr, &err);
            }
        }
        let pursuing: Vec<Address> = self.planning.pursuits.keys().cloned().collect();
        for addr in pursuing {
            if let Err(err) = self.pursue(&addr) {
                self.pursuit_refused(&addr, &err);
            }
        }
    }

    fn pursuit_refused(&mut self, addr: &Address, err: &AxError) {
        self.note(
            runtime::diagnostics::Level::Refuse,
            "kernel::pursuit",
            &format!("{} could not take work: {}", addr.as_str(), err.subject()),
        );
    }

    pub(super) fn pursue(&mut self, addr: &Address) -> Result<(), AxError> {
        while !self.flight.full() {
            let Some(state) = self.planning.pursuits.get(addr).map(kernel::Pursuit::state) else {
                return Ok(());
            };
            let busy = self.flight.rows_of(addr);
            let ready: Vec<NodeId> = self
                .ready_in(addr)
                .into_iter()
                .filter(|node| !busy.contains(node))
                .collect();
            let kernel::PursuitVerdict::Work { next } =
                kernel::pursuit::observe(state, &ready, self.flight.in_flight())
            else {
                return Ok(());
            };
            let (Some(item), Some(goal)) = (
                self.plan_item(addr, &next),
                self.planning
                    .pursuits
                    .get(addr)
                    .map(|held| held.goal().to_owned()),
            ) else {
                return Ok(());
            };
            self.note(
                runtime::diagnostics::Level::Effect,
                "kernel::pursuit",
                &format!("{} takes {next}: {item}", addr.as_str()),
            );
            let (staged, continuation) = self.stage_dispatch(
                Assignment {
                    addr: addr.clone(),
                    session: None,
                    effort: None,
                    difficulty: None,
                    model: None,
                    policy: kernel::RunPolicy::of(kernel::Mode::Work),
                    parent: None,
                    origin: None,
                    succession: None,
                    taint: kernel::TaintSet::empty(),
                    dispatched_by: kernel::event::Who::City,
                },
                format!("Plan node {next}: {item}"),
                goal,
            )?;
            self.take_row_into_lane(staged, addr.clone(), next, continuation)?;
        }
        Ok(())
    }
}
