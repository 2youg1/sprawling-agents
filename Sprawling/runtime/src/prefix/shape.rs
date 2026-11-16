// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The cache shape of one request: which bytes a prompt cache compares,
//! which region of the request moved since the request before it, and
//! how much this request can be billed for.
//!
//! Three regions make a request: the frozen system (its four segments),
//! the tool table, and the conversation this run has accumulated. A
//! cache miss is worth recording only when it says which of the three
//! moved, which is what [`PromptShape::attribute`] answers and what
//! `prompt_shape_compared` carries into the ledger.
//!
//! **The breakpoint marker is request-side only.** The breakpoint plan
//! marks one message and that mark has no serde (kernel's
//! `ChatMessage`), so the bytes any record holds are the same with and
//! without it.

use kernel::event::record::{
    PartChange, PromptAssembled, PromptShapeCompared, ShapeChanged, ShapePart,
};
use kernel::{AxCode, AxError, ChatMessage, ToolDef};
use serde::Deserialize;

use super::FrozenPrefix;
use super::segment::SegmentSlot;

/// One request's cache shape: the four frozen segments, the tool table,
/// and the conversation. The segments are the system half a prompt cache
/// matches first; the last two are the halves that grow or change after
/// a run is frozen, and they are what the frozen half exists to protect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptShape {
    segments: [ShapePart; 4],
    tools: ShapePart,
    run: ShapePart,
}

/// The bytes of one request region, measured in the canonical encoding
/// it travels in, and hashed with the one hash this city has.
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
    /// The shape of the request a turn is about to send: the frozen
    /// prefix's own hashes, the tool table's canonical JSON, and the
    /// window's canonical JSON.
    ///
    /// # Errors
    /// A region that will not serialise, or one longer than `u64` holds.
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

    /// The rebuild path: a shape reassembled from the parts a record
    /// states (`prompt_assembled` for the segments, `prompt_shape_compared`
    /// for the other two). Same type, same judgements - a replay re-derives
    /// an attribution with [`PromptShape::attribute`] rather than trusting
    /// the one that was written down.
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

    /// What moved since the request before this one, named by region and,
    /// inside the system half, by frozen segment.
    ///
    /// `None` is a run's first request: it is answered as
    /// [`ShapeChanged::FirstRequest`] rather than as a shape in which
    /// nothing moved, because nothing moved describes two shapes that
    /// agree and a first request has no shape to agree with. A reader who
    /// saw `changed: false` here would expect a cache hit that cannot
    /// happen.
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

    /// The record this shape leaves behind, with the attribution its own
    /// judgement produced and the billing bound [`PromptShape::request_upper_bound`]
    /// computed. The segments are absent from the payload for the reason
    /// `prompt_shape_compared` records: `prompt_assembled` already states
    /// them.
    ///
    /// # Errors
    /// A total past what a `u64` holds.
    pub(crate) fn recorded(&self, changed: ShapeChanged) -> Result<PromptShapeCompared, AxError> {
        Ok(PromptShapeCompared {
            tools: self.tools,
            run: self.run,
            upper_bound: self.request_upper_bound()?,
            changed,
        })
    }

    /// The most bytes this request can be billed as input: the four
    /// segments, the tool schemas and the conversation, counted in the
    /// encodings they are assembled from and added with checked
    /// arithmetic.
    ///
    /// An upper bound rather than an estimate because no token is shorter
    /// than one byte of input, so this figure never falls below the input
    /// tokens a provider may bill. The request's own framing - the model
    /// name, the role words, the JSON keys - rides on top of these bytes
    /// and is billed with them, which is why this is stated as a bound on
    /// the payload rather than as the body's exact size.
    ///
    /// # Errors
    /// A total past what a `u64` holds.
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

/// Two regions of the same request compared: did the cache's prefix
/// break here or not.
fn between(mine: ShapePart, theirs: ShapePart) -> PartChange {
    if mine == theirs {
        PartChange::Same
    } else {
        PartChange::Moved
    }
}

/// The four `prompt_assembled` rows a replay rebuilds a shape from. The
/// payload is read through the record type, so the key spellings have
/// one home - `kernel::event::record::PromptSegment`.
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
