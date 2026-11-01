// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The text a model reads for one recorded tool answer.
//!
//! The ledger keeps the structured answer; the model reads text drawn
//! from it. Two places need that text: the live wave, which pushes it
//! into the window (`turn::wave`), and the rebuild of a conversation
//! from the ledger, which a branch or a resumed session opens with
//! (`fork`). Both call [`shown`], so the two cannot draw different
//! bytes from one line.
//!
//! **Every renderer a ledger line names stays drawable.** A line records
//! the [`Rendering`] it was drawn with, and a rebuild draws it again
//! with that one, so a session written by an older build reopens with
//! the history its model actually read. `crates/runtime/spec/Rendering.lean`
//! §8-63 is the authority on each version's shape.

use kernel::event::record::{Rendering, ToolAnswer};
use kernel::{AxCode, AxError, Payload, ToolName};

mod read;
mod refusal;
mod search;

pub use read::{Copied, Numbering};

/// The renderer every answer this build writes is drawn with.
pub const CURRENT: Rendering = Rendering::V4;

/// The text `rendering` draws for one answer of the tool `name`.
///
/// A payload a renderer has no reading for is drawn as its JSON, the
/// way every answer was drawn before tools had renderers of their own:
/// the model still reads every byte, and a payload shape that moved
/// does not turn a recorded answer into a failure to rebuild.
///
/// # Errors
/// Refuses only a payload JSON will not encode, which construction
/// already rejected, so the refusal names this module as the defect.
pub fn shown(
    rendering: Rendering,
    name: &ToolName,
    answer: &ToolAnswer,
) -> Result<String, AxError> {
    match (rendering, answer) {
        (Rendering::V1, ToolAnswer::Answered { result: payload })
        | (Rendering::V1, ToolAnswer::Failed { error: payload }) => json(payload),
        (Rendering::V2 | Rendering::V3 | Rendering::V4, ToolAnswer::Failed { error }) => error
            .read::<AxError>()
            .map_or_else(|_| json(error), |refused| Ok(refusal::line(&refused))),
        (Rendering::V2 | Rendering::V3 | Rendering::V4, ToolAnswer::Answered { result }) => {
            match name.as_str() {
                crate::tools::ReadTool::NAME => read::text(
                    result,
                    Numbering::of(rendering),
                    matches!(rendering, Rendering::V4),
                )
                .map_or_else(|| json(result), Ok),
                crate::tools::SearchTool::NAME => {
                    search::text(result, Numbering::of(rendering)).map_or_else(|| json(result), Ok)
                }
                crate::tools::ToolTool::NAME => result
                    .as_map()
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .map_or_else(|| json(result), |text| Ok(text.to_owned())),
                _ => json(result),
            }
        }
    }
}

/// A payload's compact JSON: version 1 for every answer, and what
/// version 2 draws for an answer it has no reading for.
fn json(payload: &Payload) -> Result<String, AxError> {
    serde_json::to_string(payload).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, "draw a tool answer", err.to_string()).with_recovery(
            "report this against runtime::rendering: a payload refused JSON after its \
             construction already accepted it",
        )
    })
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
