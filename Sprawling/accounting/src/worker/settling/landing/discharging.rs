// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use crate::worker::{Assignment, Dispatched, Landed, Owed, Owing, RunWorker};

impl RunWorker {

    pub(in crate::worker) fn discharge(
        &mut self,
        owing: Owing,
        at: &Assignment,
        done: &Dispatched,
    ) -> Result<Landed, AxError> {
        match owing.owed() {
            Owed::Asked => Ok(Landed::Elsewhere),

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

                if handed.is_ok() && !self.collaborating.workshops.contains_key(&parent) {
                    self.knock(
                        handback.signal(),
                        &done.addr,
                        at.policy,
                        owing.knock_chain(),
                    )?;
                }

                self.record_landing(handback, done.run, &done.who)?;
                handed?;
                Ok(Landed::Elsewhere)
            }
        }
    }
}
