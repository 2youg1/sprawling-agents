// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city owed a run that has just ended, paid in one place.

use kernel::AxError;

use crate::worker::{Assignment, Dispatched, Landed, Owed, Owing, RunWorker};

impl RunWorker {
    /// Pays what the city owed the run that has just ended.
    ///
    /// The one place an entrance's obligation is settled, which is why
    /// every entrance can share one dispatch path: what differs between
    /// a person's command, a scheduled job, a knock and a delegate is
    /// this match and nothing else.
    ///
    /// # Errors
    /// Propagates a handback the parent's room will not take.
    pub(in crate::worker) fn discharge(
        &mut self,
        owing: Owing,
        at: &Assignment,
        done: &Dispatched,
    ) -> Result<Landed, AxError> {
        match owing.owed() {
            Owed::Asked => Ok(Landed::Elsewhere),
            // Nobody typed a command for this one, so the history is
            // the only place the reason can appear. A run an operator
            // did not start is the run they most need a reason for.
            Owed::Unasked(because) => {
                let because = because.because();
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "accounting::worker",
                    &format!("a run the city started itself landed, because {because}"),
                );
                Ok(Landed::Elsewhere)
            }
            Owed::Row { addr, node } => Ok(Landed::Row {
                addr: addr.clone(),
                node: node.clone(),
            }),
            Owed::Child { parent } => {
                let parent = parent.clone();
                let handback = self.deliver_handback(&parent, done)?;
                self.hand_down_what_is_ready(&parent, at, &owing)?;
                // The asker is woken by the decision every signal takes,
                // once its graph has nothing left out: each node that is
                // still out comes back on its own (sprawling-SPEC.md
                // 8-46-12).
                if !self.collaborating.workshops.contains_key(&parent) {
                    self.knock(&handback, &done.addr, at.policy, owing.knock_chain())?;
                }
                Ok(Landed::Elsewhere)
            }
        }
    }
}
