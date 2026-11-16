// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{
    PartChange, PromptAssembled, PromptShapeCompared, ShapeChanged, ShapePart,
};
use kernel::{AxCode, AxError, ChatMessage, ToolDef};
use serde::Deserialize;

use super::FrozenPrefix;
use super::segment::SegmentSlot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptShape {
    segments: [ShapePart; 4],
    tools: ShapePart,
    run: ShapePart,
}

fn part_of(bytes: Vec<u8>) -> Result<ShapePart, AxError> {
    Ok(ShapePart {
        bytes: u64::try_from(bytes.len()).map_err(|_| {
            AxError::failure(
                AxCode::InvalidArgs,
                "measure the prompt shape",
                "one request region is longer than a byte count this city can hold",
            )
            .with_recovery(
                "lower the slot byte caps in `[prefix]`, or admit fewer tools to this \
                 building's catalog",
            )
        })?,
        hash: kernel::B3Hash::digest(&bytes),
    })
}

impl PromptShape {
    pub(crate) fn of(
        prefix: &FrozenPrefix,
        tools: &[ToolDef],
        messages: &[ChatMessage],
    ) -> Result<PromptShape, AxError> {
        let [city, building, resident, run] = prefix.segments();
        let mut segments = Vec::new();
        for segment in [city, building, resident, run] {
            segments.push(ShapePart {
                bytes: u64::try_from(segment.bytes().len()).map_err(|_| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "measure the prompt shape",
                        "a frozen segment is longer than a byte count this city can hold",
                    )
                    .with_recovery("lower the slot byte caps in `[prefix]`")
                })?,
                hash: *segment.hash(),
            });
        }
        let Ok([city, building, resident, run]) = <[ShapePart; 4]>::try_from(segments) else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "measure the prompt shape",
                "a prefix is four segments and four were not measured",
            )
            .with_recovery(
                "report this against runtime::prefix: a prefix is four segments, city, \
                 building, resident and run",
            ));
        };
        Ok(PromptShape {
            segments: [city, building, resident, run],
            tools: part_of(serde_json::to_vec(&tools).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "measure the tool table",
                    err.to_string(),
                )
                .with_recovery("give every tool a schema of integers and strings only")
            })?)?,
            run: part_of(serde_json::to_vec(&messages).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "measure the conversation",
                    err.to_string(),
                )
                .with_recovery("keep tool inputs to integers and strings only")
            })?)?,
        })
    }

    pub(crate) fn from_parts(
        segments: [ShapePart; 4],
        tools: ShapePart,
        run: ShapePart,
    ) -> PromptShape {
        PromptShape {
            segments,
            tools,
            run,
        }
    }

    pub(crate) fn attribute(&self, baseline: Option<&PromptShape>) -> ShapeChanged {
        let Some(prior) = baseline else {
            return ShapeChanged::FirstRequest;
        };
        let mut system = Vec::new();
        for (slot, (mine, theirs)) in SegmentSlot::ALL
            .iter()
            .zip(self.segments.iter().zip(prior.segments.iter()))
        {
            if mine != theirs {
                system.push(slot.as_str().to_owned());
            }
        }
        ShapeChanged::Compared {
            system,
            tools: between(self.tools, prior.tools),
            run: between(self.run, prior.run),
        }
    }

    pub(crate) fn recorded(&self, changed: ShapeChanged) -> Result<PromptShapeCompared, AxError> {
        Ok(PromptShapeCompared {
            tools: self.tools,
            run: self.run,
            upper_bound: self.request_upper_bound()?,
            changed,
        })
    }

    pub(crate) fn request_upper_bound(&self) -> Result<u64, AxError> {
        let mut total = 0_u64;
        for part in self.segments.iter().chain([&self.tools, &self.run]) {
            total = total.checked_add(part.bytes).ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "bound the request",
                    "the request's byte count overflows u64",
                )
                .with_recovery("lower the slot byte caps in `[prefix]` or admit fewer tools")
            })?;
        }
        Ok(total)
    }
}

fn between(mine: ShapePart, theirs: ShapePart) -> PartChange {
    if mine == theirs {
        PartChange::Same
    } else {
        PartChange::Moved
    }
}

pub(crate) fn segment_parts_of(assembled: &serde_json::Value) -> Result<[ShapePart; 4], AxError> {
    let assembled = PromptAssembled::deserialize(assembled).map_err(|err| {
        AxError::failure(
            AxCode::CasCorrupt,
            "rebuild the prompt shape",
            format!("a prompt_assembled payload does not parse: {err}"),
        )
        .with_recovery(
            "replay with the build that wrote this ledger, or report the damaged line: its \
             prompt_assembled payload is not what this build reads",
        )
    })?;
    let mut parts = Vec::new();
    for segment in &assembled.segments {
        parts.push(ShapePart {
            bytes: segment.len,
            hash: segment.hash,
        });
    }
    let Ok(parts) = <[ShapePart; 4]>::try_from(parts) else {
        return Err(AxError::failure(
            AxCode::CasCorrupt,
            "rebuild the prompt shape",
            format!(
                "a prompt_assembled payload names {} segments, not the four a prefix has",
                assembled.segments.len()
            ),
        )
        .with_recovery(
            "replay with the build that wrote this ledger, or report the damaged line: a \
             prefix is four segments and this record states another number",
        ));
    };
    Ok(parts)
}

#[cfg(test)]
mod tests;
