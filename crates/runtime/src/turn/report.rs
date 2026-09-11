// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a completed turn hands the run loop, and the frozen `[model]`
//! section the call is shaped by.

use kernel::{Ceiling, ContentBlock, EventRef, ModelUsage, StopReason};

/// What a completed turn hands the run loop. `assistant` and
/// `wave_results` are the window-folding material — the same content the
/// ledger carries in `model_returned.data.content` and `tool_result`
/// events (C16: live folding and offline rebuild share one source).
#[derive(Debug, Clone, PartialEq)]
pub struct TurnReport {
    pub(super) refs: Vec<EventRef>,
    pub(super) model_returned: EventRef,
    pub(super) calls_made: usize,
    pub(super) assistant: Vec<ContentBlock>,
    pub(super) wave_results: Vec<ContentBlock>,
    pub(super) usage: Option<ModelUsage>,
    pub(super) stop: Option<StopReason>,
}

impl TurnReport {
    /// What the provider counted for this call, when it said. The
    /// context gauge reads `input_tokens` from here and nowhere else.
    pub fn usage(&self) -> Option<&ModelUsage> {
        self.usage.as_ref()
    }

    pub fn refs(&self) -> &[EventRef] {
        &self.refs
    }

    /// The in-window evidence candidate for `Completion::Done`.
    pub fn model_returned(&self) -> &EventRef {
        &self.model_returned
    }

    pub fn calls_made(&self) -> usize {
        self.calls_made
    }

    pub fn assistant(&self) -> &[ContentBlock] {
        &self.assistant
    }

    pub fn wave_results(&self) -> &[ContentBlock] {
        &self.wave_results
    }

    /// Why the provider stopped, when it said. A reply that stopped at
    /// the ceiling is a reply that was cut off, and the run loop reads
    /// that here rather than inferring it from what the reply contains.
    pub fn stop(&self) -> Option<StopReason> {
        self.stop
    }
}

/// Which model this run calls, how much it may say, and how hard it may
/// think — the frozen `[model]` config section as the turn sees it.
/// `effort` comes from [`kernel::FrozenConfig`] and cannot change within
/// the run: the provider renders it into the cached prompt prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallShape {
    pub model: String,
    /// The model's own ceiling, from the endpoint book. `None` when no
    /// catalogue row states one: the request then carries no ceiling and
    /// the wire that requires one refuses the call.
    pub max_tokens: Option<Ceiling>,
    pub effort: Option<kernel::Effort>,
    /// The model's window, from the endpoint book. Zero when the book
    /// does not say, in which case the context reminder stays silent.
    pub context_tokens: u64,
}
