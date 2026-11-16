// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A closure as a tool face: it answers every call as it admits it and
//! declares no effect, so its waves are serial.

use kernel::{AxCode, AxError, TimeMs, Tool, ToolCall, ToolMeta, ToolOutcome};

use super::{Admitted, ConcurrentInvoke};
use crate::bench::Ticket;

/// A closure is a tool face that answers every call as it admits it:
/// it declares no effect, so its waves are serial. citysim and the tests
/// that script a tool's answer drive a run through this one.
impl<F> ConcurrentInvoke for F
where
    F: FnMut(&ToolCall, TimeMs) -> Result<ToolOutcome, AxError>,
{
    fn meta_of(&self, _call: &ToolCall) -> Option<&ToolMeta> {
        None
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        Admitted::Answered(self(call, t))
    }

    fn tool(&self, _ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        Err(unheld_ticket())
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        _ticket: Ticket,
        _answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        Err(unheld_ticket())
    }
}

fn unheld_ticket() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "run a cleared tool call",
        "a tool face that answers every call as it admits it was handed a ticket",
    )
    .with_recovery("report this against runtime::turn::wave: only a bench issues tickets")
}
