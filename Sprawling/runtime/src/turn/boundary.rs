// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::EventRef;

use crate::conversation::Speaker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interrupt {
    None,
    Cancel,

    Steer { speaker: Speaker, text: String },
    Policy { policy: kernel::RunPolicy },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextCall {
    Allowed,

    Halted,
}

#[derive(Debug)]
pub enum PhaseOutcome<Next> {
    Advanced(Next),
    Cancelled(TurnCancelled),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnCancelled {
    pub(super) refs: Vec<EventRef>,
}

impl TurnCancelled {
    pub fn refs(&self) -> &[EventRef] {
        &self.refs
    }
}
