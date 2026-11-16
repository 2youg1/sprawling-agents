// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::ToolCall;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveCheckpoint {

    Skip,

    Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wave {

    Empty,

    ReadOnly,

    MayWrite,
}

impl Wave {

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinceCheckpoint {

    NotYet,

    Checkpointed,

    Changed,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckpointPolicy {
    since: SinceCheckpoint,
}

impl CheckpointPolicy {

    pub(crate) fn opening() -> Self {
        Self {
            since: SinceCheckpoint::NotYet,
        }
    }

    pub(crate) fn for_wave(&self, wave: Wave) -> WaveCheckpoint {
        match (self.since, wave) {
            (SinceCheckpoint::Changed, _) | (SinceCheckpoint::NotYet, Wave::MayWrite) => {
                WaveCheckpoint::Stage
            }
            (SinceCheckpoint::Checkpointed, _)
            | (SinceCheckpoint::NotYet, Wave::Empty | Wave::ReadOnly) => WaveCheckpoint::Skip,
        }
    }

    pub(crate) fn record_wave(&mut self, checkpoint: WaveCheckpoint, wave: Wave) {
        self.since = match (wave, checkpoint) {
            (Wave::MayWrite, _) => SinceCheckpoint::Changed,
            (Wave::Empty | Wave::ReadOnly, WaveCheckpoint::Stage) => SinceCheckpoint::Checkpointed,
            (Wave::Empty | Wave::ReadOnly, WaveCheckpoint::Skip) => self.since,
        };
    }
}
