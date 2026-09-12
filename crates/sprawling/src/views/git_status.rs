// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one building has written since its last fence, and where the
//! repository stands.
//!
//! **Two authorities, one answer, and they are not interchangeable.**
//! Which files moved is git's, because a working tree is what a person
//! is looking at and the Ledger records fences rather than edits. Which
//! commit was the last fence is the Ledger's, for the reason
//! `views::commits` gives: the trailers on a commit are a projection
//! for readers outside the city, and answering from them would make the
//! projection the authority.
//!
//! **The fence is the base, not the head.** `checkpoint::wave_pre`
//! files its commit under `refs/sprawling/` and leaves the head where
//! it was, so a comparison against the head would report every wave the
//! city has ever run as uncommitted work.

use kernel::Address;

use super::holding::Views;

impl Views {
    /// The working tree of one building, with the last fence behind it.
    ///
    /// `None` for a city with no repository, which is what
    /// `Unavailable` says: "there is nothing to compare against" and
    /// "nothing has changed" are different answers, and a person acts
    /// differently on each.
    pub(super) fn git_status_answer(
        &self,
        building: &Address,
    ) -> Option<channels::GitStatusAnswer> {
        // The newest commit the city fenced at this building or under
        // it, asked of the same page the commits column reads, so the
        // row shown beside the changes is the row the list opens with.
        let checkpoint = self
            .commits_answer(Some(building), None, 1)
            .commits
            .into_iter()
            .next();
        let status = memory::working_status(
            &self.city_root,
            Some(building.as_str()),
            checkpoint.as_ref().map(|commit| commit.oid),
        )
        .ok()?;
        Some(channels::GitStatusAnswer {
            building: building.clone(),
            branch: status.branch,
            drift: status.drift.map(|drift| channels::Drift {
                ahead: drift.ahead,
                behind: drift.behind,
            }),
            files: status.files,
            checkpoint,
        })
    }
}
