// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What is waiting for a person, and what they have already answered.
//!
//! Two arms of one fold, in one file because they are two halves of one
//! subject: an item arrives, and later a ruling closes it. Reading the
//! two side by side is what shows that the question is stored whole and
//! the answer is stored as the ruling plus the cluster it covered.

use kernel::{AxError, EventRecord};

use super::holding::Views;

impl Views {
    /// Files one question that is waiting for a person.
    ///
    /// The payload *is* the item: it was written by serialising one, so
    /// it reads back as one. Rebuilding a lesser shape out of
    /// hand-picked fields is how this view came to show every waiting
    /// item as "(no summary recorded)" — the field it read had never
    /// been written by anybody.
    ///
    /// Read through the same [`kernel::ApprovalItem`] the worker's own
    /// fold reads, so the two sides of one line cannot disagree about
    /// whether it is an approval.
    ///
    /// # Errors
    /// Refuses a payload that is not an approval item. The record
    /// stands; this view skips it and the observer reports it.
    pub(super) fn fold_question(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let item = record.data().read::<kernel::ApprovalItem>()?;
        self.approvals.insert(item.id.as_str().to_owned(), item);
        Ok(())
    }

    /// Closes one question with the ruling a person gave it.
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
        self.approvals.remove(ruled.id.as_str());
        self.decided.push(channels::Decision {
            item: ruled.id.as_str().to_owned(),
            verdict: ruled.verdict,
            cluster: ruled.cluster,
            at: record.t(),
        });
        Ok(())
    }
}
