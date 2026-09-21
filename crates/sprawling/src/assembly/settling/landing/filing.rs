// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Putting a question into the Approval Inbox.
//!
//! Separate from the landing itself because the two answer to
//! different readers: a landing is read by the run that asked for the
//! work, and a filed question is read by the person who has to answer
//! it before that work goes on.

use kernel::{AxCode, AxError, EventKind, Payload};

use super::Filing;
use crate::assembly::RunWorker;
use crate::effect;

impl RunWorker {
    /// Files what the drive raised into the Approval Inbox.
    ///
    /// Recorded against the run and the address that raised it rather
    /// than against the city: answering one of these later has to find
    /// the work it was holding up.
    ///
    /// # Errors
    /// Propagates a ledger that will not take the line.
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
        // An item that is waiting belongs in the inbox, not only in the
        // refusal the model saw.
        // Recorded against the run and the address that raised it, not
        // against the city: answering this item later has to be able to
        // find the work it was holding up.
        if tainted {
            // C15's marker bit, set where the reason for it is known. A
            // tainted item takes no policy and no delegate, so a run that
            // began with a stranger's text cannot have its approvals
            // waived by a rule somebody wrote for ordinary work.
            for item in raised.iter_mut() {
                item.tainted = true;
            }
        }
        for item in raised.iter() {
            let value = serde_json::to_value(item).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "record a waiting item",
                    err.to_string(),
                )
                .with_recovery(
                    "report this against sprawling::assembly::settling::landing: an \
                     approval item is text, flags and one address",
                )
            })?;
            let map = value.as_object().cloned().ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "record a waiting item",
                    "an approval item is an object",
                )
                .with_recovery(
                    "report this against sprawling::assembly::settling::landing: an \
                     approval item encodes as a JSON object and this one did not",
                )
            })?;
            self.record_for(
                run_id,
                effect::Line {
                    who: who.to_owned(),
                    addr: addr.clone(),
                    kind: EventKind::ApprovalRequested,
                    data: Payload::new(map)?,
                },
            )?;
        }
        Ok(())
    }
}
