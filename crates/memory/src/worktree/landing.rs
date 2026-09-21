// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A merge that has been decided, and what it writes down when it is
//! made.

use kernel::TimeMs;

use crate::checkpoint::Provenance;
use crate::error::MemoryError;

use super::trees::Worktrees;

/// What one merge writes down: four values that arrive together and are
/// meaningless apart - a subject with nobody's provenance under it says
/// who merged nothing. `reviewed_by_person` adds one `Reviewed-by: Name
/// <email>` trailer, and only when this repository's git config also
/// carries `user.name` and `user.email`: the city does not invent a
/// person's name.
pub struct Landing<'a> {
    /// Determinism rule 2: the signature carries the injected instant.
    pub t: TimeMs,
    pub of: &'a Provenance,
    pub subject: &'a str,
    pub reviewed_by_person: bool,
}

/// A merge that has been decided and not yet made.
///
/// The commit the trunk will land on is settled at construction and
/// every refusal has already happened, so the line announcing this merge
/// can be written before the trunk moves. [`PlannedMerge::apply`] is the
/// only way to move it, and [`Worktrees::plan_merge`] is this value's
/// only source.
pub struct PlannedMerge<'a> {
    /// Reachable from `trees` alone: [`Worktrees::plan_merge`] is this
    /// value's only source, and the refusals it made are why holding
    /// one means the merge was allowed.
    pub(super) trees: &'a Worktrees,
    pub(super) target: git2::Oid,
}

/// Names the decision, not the repository holding it: a `Worktrees` has
/// no useful `Debug` and printing one would say nothing about which
/// merge this is.
impl std::fmt::Debug for PlannedMerge<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannedMerge")
            .field("commit", &self.target)
            .finish()
    }
}

impl PlannedMerge<'_> {
    /// The commit the city trunk will point at, for the line that says so.
    pub fn commit(&self) -> String {
        self.target.to_string()
    }

    /// Brings a node's committed work into the city's own trunk, as a
    /// merge commit carrying the merging run's trailers. The judgement
    /// stays fast-forward only; what the history keeps is the merge
    /// commit, because a pointer move leaves nothing to read and
    /// no place to say who verified it.
    ///
    /// # Errors
    /// Propagates a trunk that cannot be read, committed onto or checked
    /// out. The fast-forward judgement is not repeated: it was made, and
    /// refused if it had to be, before this value existed.
    pub fn apply(self, landing: &Landing<'_>) -> Result<(), MemoryError> {
        self.trees.land_merge(self.target, landing)
    }
}
