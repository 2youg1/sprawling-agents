// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run has recorded of its prompt.
//!
//! `prompt_assembled` is written once per run: on the first turn, and
//! again only when the payload a turn assembles differs from the last
//! one written. The prefix is frozen for the run and the breakpoint plan
//! moves only when the conversation goes from empty to not, so every
//! later line was a copy of the first; what does move between turns is
//! recorded by `prompt_shape_compared`. The comparison is on the payload
//! rather than on the turn index, so a turn whose request really differs
//! still gets its own line (`crates/runtime/spec/Prefix.lean` §8-39, item 5).

use kernel::Payload;

use crate::prefix::FrozenPrefix;

/// The prompt one turn is assembled from: the run's frozen prefix, and
/// what the run has already recorded of it.
#[derive(Debug)]
pub struct RunPrompt<'r> {
    pub(super) prefix: &'r FrozenPrefix,
    pub(super) recorded: &'r mut PromptRecord,
}

impl<'r> RunPrompt<'r> {
    pub fn new(prefix: &'r FrozenPrefix, recorded: &'r mut PromptRecord) -> RunPrompt<'r> {
        RunPrompt { prefix, recorded }
    }
}

/// The `prompt_assembled` payload a run wrote last; empty before its
/// first turn. It lives across turns, in the run's own state.
#[derive(Debug, Default)]
pub struct PromptRecord {
    written: Option<Payload>,
}

impl PromptRecord {
    /// Whether `payload` is what this run last wrote, so writing it again
    /// would add a copy rather than a fact.
    pub(super) fn holds(&self, payload: &Payload) -> bool {
        self.written.as_ref() == Some(payload)
    }

    /// Remembers `payload` as the line this run wrote last.
    pub(super) fn remember(&mut self, payload: Payload) {
        self.written = Some(payload);
    }
}
