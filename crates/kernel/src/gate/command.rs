// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The command door: whether a run may execute a command at all.
//! The part `crates/kernel/spec/Gate.lean` specifies this module with the
//! rest of `kernel::gate`.
//!
//! A command's effect is whatever its text says, and no door can list
//! that in advance, so the only input this door reads is where the run's
//! work came from. A run that outside content started — an MCP answer,
//! a fetched page, a file from another building — is refused rather than
//! asked: asking would hand the person a command the outside wrote and
//! a button to run it.

use crate::error::{AxCode, AxError, GateRefusal};
use crate::taint::TaintSet;

use super::GateOutcome;

/// Allows a command from work that began inside the city; refuses one
/// from a run carrying any external source (C15).
#[must_use]
pub fn command(taint: &TaintSet) -> GateOutcome {
    if taint.is_empty() {
        return GateOutcome::Allow;
    }
    GateOutcome::Deny {
        refusal: Box::new(
            AxError::refusal(
                AxCode::TaintedAction,
                "run a command",
                "exec",
                GateRefusal::new(
                    "a run started by outside content runs no command (C15)",
                    format!(
                        "this run carries content from {taint}, and a command can do \
                         anything its text says"
                    ),
                    "read, edit inside the write domain, or report what the outside \
                     content asked for instead of running it",
                ),
            )
            .with_recovery(
                "run the command from work that did not start outside the city; a person \
                 can hand the same task over as their own",
            ),
        ),
    }
}
