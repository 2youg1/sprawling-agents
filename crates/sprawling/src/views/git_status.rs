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

//! **The walk of the disk happens after the views are released.** The
//! views hand over only the fence and the root; `git status` over a
//! large tree takes tens of milliseconds, and the fold waits for every
//! one of them it is run under (sprawling-SPEC.md 8-92).

use std::path::PathBuf;

use kernel::Address;

use super::holding::Views;

/// What `git status` for one building needs from the views, copied out
/// while they are held.
pub(crate) struct GitStatusAsk {
    city_root: PathBuf,
    building: Address,
    checkpoint: Option<channels::CommitAnswer>,
}

impl Views {
    /// The building's last fence and the city root, for a read of the
    /// working tree after the views are released.
    /// `None` when the newest fence's row cannot be read, which the
    /// caller answers as `Unavailable`, as the commits column does.
    pub(super) fn git_status_ask(&self, building: &Address) -> Option<GitStatusAsk> {
        Some(GitStatusAsk {
            city_root: self.city_root.clone(),
            building: building.clone(),
            // The newest commit the city fenced at this building or
            // under it, asked of the same page the commits column
            // reads, so the row shown beside the changes is the row the
            // list opens with.
            checkpoint: self
                .commits_answer(Some(building), None, 1)?
                .commits
                .into_iter()
                .next(),
        })
    }
}

impl GitStatusAsk {
    /// The working tree of one building, with the last fence behind it.
    ///
    /// `Unavailable` for a city with no repository: "there is nothing to
    /// compare against" and "nothing has changed" are different answers,
    /// and a person acts differently on each.
    pub(super) fn read(self) -> channels::Answer {
        let Ok(status) = memory::working_status(
            &self.city_root,
            Some(self.building.as_str()),
            self.checkpoint.as_ref().map(|commit| commit.oid),
        ) else {
            return channels::Answer::Unavailable {
                query: format!("GitStatus({})", self.building.as_str()),
            };
        };
        channels::Answer::GitStatus(Box::new(channels::GitStatusAnswer {
            building: self.building,
            branch: status.branch,
            drift: status.drift.map(|drift| channels::Drift {
                ahead: drift.ahead,
                behind: drift.behind,
            }),
            files: status.files,
            checkpoint: self.checkpoint,
        }))
    }
}
