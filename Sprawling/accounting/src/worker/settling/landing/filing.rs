// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, EventKind, Payload};

use super::Filing;
use crate::effect;
use crate::worker::RunWorker;

impl RunWorker {

    pub(super) fn file_the_waiting(
        &mut self,
        at: &Filing<'_>,
        raised: &mut [kernel::ApprovalItem],
    ) -> Result<(), AxError> {
        let Filing {
            run_id,
            addr,
            who,
            tainted,
        } = *at;

        if tainted {

            for item in raised.iter_mut() {
                item.tainted = true;
            }
        }
        for item in raised.iter() {
            self.record_for(
                run_id,
                effect::Line {
                    who: who.to_owned(),
                    addr: addr.clone(),
                    kind: EventKind::ApprovalRequested,
                    data: Payload::of(item)?,
                },
            )?;
        }
        Ok(())
    }
}
