// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{Rendering, ToolAnswer};
use kernel::{AxCode, AxError, Payload, ToolName};

mod read;
mod refusal;
mod search;

pub use read::{Copied, Numbering, rule_overhead};
pub(crate) use read::{Stub, unchanged};

pub const CURRENT: Rendering = Rendering::V4;

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
                crate::tools::ReadTool::NAME if rendering == Rendering::V4 => {
                    read::text_v4(result).map_or_else(|| json(result), Ok)
                }
                crate::tools::ReadTool::NAME => {
                    read::text(result, Numbering::of(rendering)).map_or_else(|| json(result), Ok)
                }
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
