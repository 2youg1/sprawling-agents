// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The breakpoint plan: where one request's explicit prompt-cache
//! breakpoints sit.
//!
//! **One author.** The system blocks, the conversation and the
//! `prompt_assembled` record all read their breakpoints from this plan,
//! so a record cannot name a breakpoint the request does not carry. A
//! dialect only spells the marks it is handed; it decides none of them.

use kernel::{ChatMessage, MessageBreakpoint};

use super::segment::SegmentSlot;

/// One explicit prompt-cache breakpoint a request carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// The end of a frozen segment.
    Edge(SegmentSlot),
    /// The last message of the conversation, whatever its role.
    Tail,
}

impl Breakpoint {
    /// The spelling `prompt_assembled` records: the slot name for an
    /// edge, `tail` for the conversation's last message.
    pub fn as_str(self) -> &'static str {
        match self {
            Breakpoint::Edge(slot) => slot.as_str(),
            Breakpoint::Tail => "tail",
        }
    }
}

/// The breakpoints one request carries: every segment edge but the
/// last, and the tail of the conversation when there is one.
///
/// The run segment's edge carries none because the tail anchor sits
/// right after it and covers it, and a fifth breakpoint is one the
/// provider refuses (`CACHE_BREAKPOINTS_MAX`). The tail is the last
/// message rather than the last assistant message: trailing tool results
/// ride in the last message, and anchoring before them pays for them
/// again on every request (`crates/runtime/spec/Turn.lean` §8-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakpointPlan {
    tail: Option<usize>,
}

impl BreakpointPlan {
    /// The plan for a request carrying `messages`.
    pub fn for_conversation(messages: &[ChatMessage]) -> BreakpointPlan {
        BreakpointPlan {
            tail: messages.len().checked_sub(1),
        }
    }

    /// Whether the end of `slot` carries a breakpoint. The edges do not
    /// depend on the conversation, so a request built without one (a
    /// probe) asks the same question.
    pub fn marks_edge(slot: SegmentSlot) -> bool {
        match slot {
            SegmentSlot::City | SegmentSlot::Building | SegmentSlot::Resident => true,
            SegmentSlot::Run => false,
        }
    }

    /// The message breakpoint a request carries under this plan, so a
    /// request carries exactly the message breakpoint the plan names.
    pub fn message_breakpoint(&self) -> MessageBreakpoint {
        match self.tail {
            Some(_) => MessageBreakpoint::Tail,
            None => MessageBreakpoint::Unmarked,
        }
    }

    /// Every breakpoint in request order: the edges, then the tail.
    pub fn breakpoints(&self) -> Vec<Breakpoint> {
        SegmentSlot::ALL
            .into_iter()
            .filter(|slot| BreakpointPlan::marks_edge(*slot))
            .map(Breakpoint::Edge)
            .chain(self.tail.map(|_| Breakpoint::Tail))
            .collect()
    }
}
