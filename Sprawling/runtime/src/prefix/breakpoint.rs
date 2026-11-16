// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{ChatMessage, MessageBreakpoint};

use super::segment::SegmentSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {

    Edge(SegmentSlot),

    Tail,
}

impl Breakpoint {

    pub fn as_str(self) -> &'static str {
        match self {
            Breakpoint::Edge(slot) => slot.as_str(),
            Breakpoint::Tail => "tail",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakpointPlan {
    tail: Option<usize>,
}

impl BreakpointPlan {

    pub fn for_conversation(messages: &[ChatMessage]) -> BreakpointPlan {
        BreakpointPlan {
            tail: messages.len().checked_sub(1),
        }
    }

    pub fn marks_edge(slot: SegmentSlot) -> bool {
        match slot {
            SegmentSlot::City | SegmentSlot::Building | SegmentSlot::Resident => true,
            SegmentSlot::Run => false,
        }
    }

    pub fn message_breakpoint(&self) -> MessageBreakpoint {
        match self.tail {
            Some(_) => MessageBreakpoint::Tail,
            None => MessageBreakpoint::Unmarked,
        }
    }

    pub fn breakpoints(&self) -> Vec<Breakpoint> {
        SegmentSlot::ALL
            .into_iter()
            .filter(|slot| BreakpointPlan::marks_edge(*slot))
            .map(Breakpoint::Edge)
            .chain(self.tail.map(|_| Breakpoint::Tail))
            .collect()
    }
}
