// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one tool call records: what was asked, and what came back.

use serde::{Deserialize, Serialize};

use crate::event::payload::Payload;
use crate::tool::ToolName;

/// `tool_called`: the call the model asked for, before anything ran.
///
/// The arguments reach the ledger through
/// `runtime::turn::ledger::Journal::append_redacted`, so a credential
/// the model quoted into them is replaced before this line exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolCalled {
    /// The wire identity both dialects pair a call and its result by.
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub name: ToolName,
    pub args: Payload,
}

/// `tool_result`: what the call returned, under the id it answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolResult {
    /// The `tool_called` id this line answers.
    pub tool_use_id: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub name: ToolName,
    #[serde(flatten)]
    pub answer: ToolAnswer,
}

/// A call answers exactly one way, and the enum is what says so: two
/// independent optional keys would let a line hold both or neither, and
/// every reader would have to invent a verdict for the two shapes that
/// cannot happen.
///
/// Untagged, because the keys are the discriminator the hand-written
/// writer left behind and a ledger already on disk cannot be
/// re-spelled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ToolAnswer {
    /// The tool answered; the payload is the tool's own result shape.
    Answered { result: Payload },
    /// The tool refused or failed; the payload is the serialized
    /// `AxError`, which carries the code, the subject and the recovery
    /// the model is expected to act on.
    Failed { error: Payload },
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
