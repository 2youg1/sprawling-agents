// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether a wave needs a checkpoint before it runs — the one
//! authority on that question (runtime-SPEC §8-45).

use kernel::ToolCall;

/// What goes up before one wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveCheckpoint {
    /// The last commit already holds the tree this wave starts from.
    Skip,
    /// Stage the write domain and commit before the first call runs.
    Stage,
}

/// What one wave is about to do to the tree, by its calls' declared effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wave {
    /// No calls.
    Empty,
    /// Every call answers `Writes::Nothing`: the tree stays as it is.
    ReadOnly,
    /// At least one call may write.
    MayWrite,
}

impl Wave {
    /// Classifies `calls` by what `writes` answers for each.
    pub(crate) fn of(calls: &[ToolCall], writes: &dyn Fn(&ToolCall) -> kernel::Writes) -> Wave {
        match calls {
            [] => Wave::Empty,
            [_, ..]
                if calls
                    .iter()
                    .all(|call| writes(call) == kernel::Writes::Nothing) =>
            {
                Wave::ReadOnly
            }
            [_, ..] => Wave::MayWrite,
        }
    }
}

/// What the run's tree is, measured against the last checkpoint this run put up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinceCheckpoint {
    /// No checkpoint yet and no writing call run: the tree is the one the run
    /// opened on.
    NotYet,
    /// A checkpoint went up and no writing call has run since, so it holds
    /// this tree.
    Checkpointed,
    /// A call that may write ran after the last checkpoint (or before any), so
    /// the tree may hold writes no commit carries yet.
    Changed,
}

/// Decides the checkpoint for each wave of one run, from the calls the wave is
/// about to make and what ran since the last checkpoint.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckpointPolicy {
    since: SinceCheckpoint,
}

impl CheckpointPolicy {
    /// The policy of a run that has not taken a turn.
    pub(crate) fn opening() -> Self {
        Self {
            since: SinceCheckpoint::NotYet,
        }
    }

    /// A wave after calls that may have written is staged even when it
    /// calls nothing itself, because the checkpoint is also what carries those
    /// writes into a commit; a wave whose tree the last checkpoint already holds,
    /// or the run opened on, is skipped until a call may write (§8-45).
    pub(crate) fn for_wave(&self, wave: Wave) -> WaveCheckpoint {
        match (self.since, wave) {
            (SinceCheckpoint::Changed, _) | (SinceCheckpoint::NotYet, Wave::MayWrite) => {
                WaveCheckpoint::Stage
            }
            (SinceCheckpoint::Checkpointed, _)
            | (SinceCheckpoint::NotYet, Wave::Empty | Wave::ReadOnly) => WaveCheckpoint::Skip,
        }
    }

    /// Records the wave about to run under `checkpoint`; a wave cut short by a
    /// cancel still counts as one that ran, since some of its calls may have.
    pub(crate) fn record_wave(&mut self, checkpoint: WaveCheckpoint, wave: Wave) {
        self.since = match (wave, checkpoint) {
            (Wave::MayWrite, _) => SinceCheckpoint::Changed,
            (Wave::Empty | Wave::ReadOnly, WaveCheckpoint::Stage) => SinceCheckpoint::Checkpointed,
            (Wave::Empty | Wave::ReadOnly, WaveCheckpoint::Skip) => self.since,
        };
    }
}
