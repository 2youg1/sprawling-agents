// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one tool call records: what was asked, and what came back.

use serde::{Deserialize, Serialize};

use crate::event::payload::Payload;
use crate::tool::ToolName;

/// Argument names that say what a call acted on, in the order they are
/// preferred. Taken from the tool definitions rather than guessed:
/// `path` is what most of them take, and the rest name their one
/// subject.
const SUBJECT_KEYS: [&str; 4] = ["path", "addr", "program", "arm"];

/// `tool_called`: the call the model asked for, before anything ran.
///
/// The arguments, and the subject read from them, reach the ledger
/// through `runtime::turn::ledger::Journal::append_redacted`, so a
/// credential the model quoted into them is replaced before this line
/// exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolCalled {
    /// The wire identity both dialects pair a call and its result by.
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub name: ToolName,
    pub args: Payload,
    /// What the call acted on, in the one reading a person recognises
    /// a call by: a path, an address, a program name.
    ///
    /// Carried by the record rather than derived from `args` at each
    /// reader. Two readers deriving it each iterated their own map
    /// order, so one call answered differently on the two sides of the
    /// wire: for `{"10": "a", "2": "b"}` the ledger's byte order
    /// gives `"a"` and a browser's own-property order gives `"b"`.
    /// [`ToolCalled::subject_of`] decides it once, here.
    ///
    /// `None` is a call whose arguments name nothing of that kind,
    /// which is a real state rather than a failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

impl ToolCalled {
    /// The one argument a person recognises a call by, read from the
    /// arguments once, when the call becomes a record.
    ///
    /// The preferred keys are tried in their declared order; a call
    /// whose tool names none of them still says something rather than
    /// falling back to the bare tool name, and the fallback takes the
    /// first string in the payload's own key order so it is the same
    /// answer on every machine.
    #[must_use]
    pub fn subject_of(args: &Payload) -> Option<String> {
        for key in SUBJECT_KEYS {
            if let Some(named) = args.as_map().get(key).and_then(serde_json::Value::as_str) {
                return Some(named.to_owned());
            }
        }
        args.as_map()
            .values()
            .find_map(serde_json::Value::as_str)
            .map(str::to_owned)
    }
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
