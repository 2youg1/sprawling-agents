// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one tool call records: what was asked, and what came back.

use serde::{Deserialize, Serialize};

use crate::event::payload::Payload;
use crate::tool::{Effect, RenderIntent, ToolName};

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
    /// The effect the tool was registered with at the moment of the
    /// call, copied by the tool wave from its `ToolMeta` (`crates/kernel/spec/Event/Record.lean`
    /// §8-75). Absent for a tool the bench did not know and on every line
    /// written before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Effect>,
    /// How the tool was registered to be drawn at the moment of the
    /// call; absent in the same cases as `effect`. A `Diff`'s locations
    /// are the registration's, empty for every tool today: the file an
    /// edit touched is `subject`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<RenderIntent>,
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
    /// Whole microseconds from the moment the call started to the moment
    /// it answered, read off the turn's monotonic clock
    /// (`crates/kernel/spec/Event/Record.lean` D20). Absent on every line
    /// written before the key existed: a reader then falls back to the
    /// difference of the two envelope moments, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_us: Option<u64>,
    /// The renderer that drew the text the model read for this answer
    /// (`crates/kernel/spec/Event/Record.lean` D64). Absent on every line
    /// written before the key existed, which reads as [`Rendering::V1`]:
    /// those lines were all shown to a model as their payload's JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered: Option<Rendering>,
    /// How many bytes of text the model was shown for this answer, after
    /// the pipeline shortened it and the renderer drew it. Absent where it
    /// was not counted, which is every line written before the key
    /// existed and every answer the city wrote rather than a tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown_bytes: Option<u64>,
}

/// Which renderer turned a recorded answer into the text a model read.
///
/// The ledger keeps the structured answer and the model reads text, so
/// the text is a function of the record and of the renderer. A branch
/// rebuilt from the ledger has to show the model what it was shown when
/// the line was written, so the line names the renderer and every
/// version stays drawable for as long as a ledger holds a line it drew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Rendering {
    /// The payload's compact JSON, which every answer was shown as before
    /// a line could name its renderer.
    V1,
    /// One header line and a body for the tools that have one, and a
    /// refusal as one line (`crates/runtime/spec/Rendering.lean` §8-63).
    V2,
}

impl Rendering {
    /// The renderer a line names, reading an absent key as the renderer
    /// every line was drawn with before the key existed.
    #[must_use]
    pub fn of(recorded: Option<Rendering>) -> Rendering {
        recorded.unwrap_or(Rendering::V1)
    }
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
