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
    /// Nothing the wave does can need a commit to come back from.
    Skip,
    /// Stage the write domain and commit before the first call runs.
    Stage,
}

/// Decides the fence for a wave from the calls it is about to make.
pub(crate) struct FencePolicy;

impl FencePolicy {
    /// An empty wave changes nothing, so it is skipped; any other wave
    /// is staged, because the run driver does not yet know each call's
    /// declared effect (§8-45 names the read-only case as open).
    pub(crate) fn for_wave(calls: &[ToolCall]) -> Fence {
        match calls {
            [] => Fence::Skip,
            [_, ..] => Fence::Stage,
        }
    }
}
