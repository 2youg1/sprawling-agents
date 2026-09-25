// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether a wave needs a checkpoint fence before it runs — the one
//! authority on that question (runtime-SPEC §8-45).

use kernel::ToolCall;

/// What goes up before one wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fence {
    /// The last commit already holds the tree this wave starts from.
    Skip,
    /// Stage the write domain and commit before the first call runs.
    Stage,
}

/// What the run's tree is, measured against the last fence this run put up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinceFence {
    /// No fence yet and no call run: the tree is the one the run opened on.
    Unfenced,
    /// A fence went up and no call has run since, so it holds this tree.
    Fenced,
    /// A call ran after the last fence (or before any), so the tree may
    /// hold writes no commit carries yet.
    Changed,
}

/// Decides the fence for each wave of one run, from the calls the wave is
/// about to make and what ran since the last fence.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FencePolicy {
    since: SinceFence,
}

impl FencePolicy {
    /// The policy of a run that has not taken a turn.
    pub(crate) fn opening() -> Self {
        Self {
            since: SinceFence::Unfenced,
        }
    }

    /// A wave after calls that may have written is staged even when it
    /// calls nothing itself, because the fence is also what carries those
    /// writes into a commit; a wave whose tree the last fence already holds
    /// is skipped. Every call counts as one that may write, because the run
    /// driver does not yet know each call's declared effect (§8-45).
    pub(crate) fn for_wave(&self, calls: &[ToolCall]) -> Fence {
        match (self.since, calls) {
            (SinceFence::Changed, _) | (SinceFence::Unfenced, [_, ..]) => Fence::Stage,
            (SinceFence::Fenced, _) | (SinceFence::Unfenced, []) => Fence::Skip,
        }
    }

    /// Records the wave about to run under `fence`; a wave cut short by a
    /// cancel still counts as one that ran, since some of its calls may have.
    pub(crate) fn record_wave(&mut self, fence: Fence, calls: &[ToolCall]) {
        self.since = match (calls, fence) {
            ([_, ..], _) => SinceFence::Changed,
            ([], Fence::Stage) => SinceFence::Fenced,
            ([], Fence::Skip) => self.since,
        };
    }
}
