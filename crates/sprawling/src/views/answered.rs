// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person has already answered.
//!
//! What is *waiting* for one is `views::governance`'s, and this file
//! only records the answer: an answered item leaves the queue there and
//! joins the list here, and the two halves are held apart because the
//! queue is a state the worker judges from and the list is a history
//! nobody judges from.

use kernel::{AxError, EventRecord};

use super::holding::Views;

impl Views {
    /// Records the ruling a person gave one question.
    ///
    /// The cluster travels with the answer because the person answered
    /// the group they were shown, and it is read back through the same
    /// [`kernel::event::record::ApprovalResolved`] the answer was
    /// written from. Reading the three fields by hand here is how this
    /// view came to record an empty cluster for a group it could not
    /// parse while the worker's fold, reading the same line, granted
    /// nothing at all.
    ///
    /// # Errors
    /// Refuses a ruling this build cannot read. An unknown verdict used
    /// to read as an allowance, which showed the person a permission
    /// they had never given.
    pub(super) fn fold_ruling(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let ruled = record
            .data()
            .read::<kernel::event::record::ApprovalResolved>()?;
        self.decided.push(channels::Decision {
            item: ruled.id.as_str().to_owned(),
            verdict: ruled.verdict,
            cluster: ruled.cluster,
            at: record.t(),
        });
        Ok(())
    }
}
