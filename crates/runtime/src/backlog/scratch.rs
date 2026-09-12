// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where one member of one backlog keeps its output while it runs.
//!
//! **A [`BacklogId`] cannot name this directory, and that is the whole
//! reason this module has a name of its own.** An id identifies a member
//! inside one backlog and starts again at one in the next; a directory
//! sits in a temp directory every backlog on the machine shares. Naming
//! it `sprawling-<pid>-<id>` assumed the id was unique per process, and
//! two backlogs of one process then collided on their first command each
//! (runtime-SPEC.md 8-40).

use std::sync::atomic::{AtomicU64, Ordering};

use super::BacklogId;

/// How many backlogs this process has opened.
static BACKLOGS: AtomicU64 = AtomicU64::new(0);

/// One backlog's claim on the machine's temp directory.
///
/// `Copy`, and carried by every clone of a [`Backlog`](super::Backlog):
/// two handles onto one backlog keep naming its directories, while two
/// backlogs opened separately never name each other's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Scratch(u64);

impl Scratch {
    /// The next claim this process has not used.
    pub(super) fn open() -> Scratch {
        // Relaxed is enough: nothing is published through this number,
        // and all it has to be is different from every other one.
        Scratch(BACKLOGS.fetch_add(1, Ordering::Relaxed))
    }

    /// All three parts are load-bearing: two cities share one temp
    /// directory, one process may hold several backlogs whose ids both
    /// start at one, and one backlog runs many members.
    pub(super) fn dir(self, id: BacklogId) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sprawling-{}-{}-{}",
            std::process::id(),
            self.0,
            id
        ))
    }
}
