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
    /// Propagates a handback the parent's room will not take, and the
    /// ledger's refusal of where it landed.
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
                let handed = self.hand_down_what_is_ready(&parent, at, &owing);
                // The asker is woken by the decision every signal takes,
                // once its graph has joined or been closed: each node that
                // is still out comes back on its own (collab §8
                // `collab::handback`, `crates/collab/spec/Workshop.lean`).
                if handed.is_ok() && !self.collaborating.workshops.contains_key(&parent) {
                    self.knock(
                        handback.signal(),
                        &done.addr,
                        at.policy,
                        owing.knock_chain(),
                    )?;
                }
                // The child wrote the letter, so its landing is the child's
                // line too, and it is written even when handing down the
                // next nodes was refused: the letter is in the room either
                // way (kernel D38).
                self.record_landing(handback, done.run, &done.who)?;
                handed?;
                Ok(Landed::Elsewhere)
            }
        }
    }
}
