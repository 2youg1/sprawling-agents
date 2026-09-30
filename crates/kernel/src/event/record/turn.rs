// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one turn records: the prefix it assembled, the call it sent,
//! the reply that came back, and the steer that arrived at a boundary.

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::budget::UsdMicros;
use crate::event::identity::TimeMs;
use crate::event::payload::Payload;
use crate::locator::B3Hash;
use crate::model::{ModelUsage, StopReason};

/// `prompt_assembled`: the four segments a request was built from, and
/// the cache breakpoints it carried.
///
/// This is the line an offline rebuild reads (A15): every segment says
/// which documents went into it and how much of each one survived the
/// slot's byte cap, so the bytes the model read can be reconstructed
/// from the files without the request being kept.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptAssembled {
    #[serde(default)]
    pub segments: Vec<PromptSegment>,
    /// The explicit cache breakpoints the request carried, in request
    /// order: a slot name for a segment edge, `tail` for the last
    /// message. The plan has one author, `runtime::prefix::BreakpointPlan`,
    /// and this key carries what that author printed. A ledger written
    /// before the plan existed holds the four slot names here instead.
    #[serde(default)]
    pub breakpoints: Vec<String>,
}

/// One frozen segment, as `prompt_assembled` states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptSegment {
    pub slot: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,
    /// Bytes this segment contributed to the request.
    pub len: u64,
    /// The documents behind the segment, in concatenation order.
    /// Written even when empty: a segment built from an identity rather
    /// than from files names none, and that is a fact rather than an
    /// omission.
    #[serde(default)]
    pub sources: Vec<PromptSource>,
    /// The documents the slot was offered and did not take.
    #[serde(default)]
    pub skipped: Vec<PromptSkip>,
}

/// One document a segment was assembled from.
///
/// `kept` and `dropped` state both what the model read and what the cap
/// cut off; `marker` says a truncation marker follows the kept bytes,
/// which the byte counts alone cannot say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptSource {
    pub addr: Address,
    pub kept: u64,
    pub marker: bool,
    pub dropped: u64,
}

/// One document a slot was offered and left out, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptSkip {
    pub addr: Address,
    pub reason: SkipReason,
}

/// Why a document did not reach its segment. Closed: an assembler that
/// finds a fifth reason states it here, where every reader sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkipReason {
    /// An earlier slot already carried this document; the first slot
    /// wins, because the earlier slot is the more cacheable one.
    Duplicate,
    /// The resolver returned no bytes for the address.
    Unreadable,
    /// The bytes are not UTF-8, and a prompt is text.
    NotUtf8,
    /// The slot's byte cap was already spent.
    NoBudget,
}

/// `model_called`: which prefix went out, under which model name.
///
/// The request body is deliberately absent: it is reconstructible from
/// [`PromptAssembled`] plus the window, and a copy of it here would be
/// the second home of every message the run ever sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelCalled {
    /// The four segment hashes, in prefix order.
    #[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
    pub segments: Vec<B3Hash>,
    /// The endpoint's own model id, as the request spelled it.
    pub model: String,
}

/// `model_returned`: the reply, the size of the wave it asked for, and
/// what the provider said it cost.
///
/// Every field but `message` and `calls` is absent when the provider
/// did not report it, which is not the same fact as reporting zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelReturned {
    /// The assistant message, in the canonical content-block shape.
    pub message: Payload,
    /// How many tool calls the reply asked for.
    pub calls: u64,
    /// Four token counts in the one meaning `ModelUsage` states, the
    /// dialect that reported them, and the meaning's version `v`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<std::collections::BTreeMap<String, serde_json::Value>>")
    )]
    pub usage: Option<ModelUsage>,
    /// Why the provider stopped, when it said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub stop: Option<StopReason>,
    /// What the provider itself billed, when it returns a figure; the
    /// city's own price list is what fills the gap when it does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billed_usd_micros: Option<UsdMicros>,
    /// When the reply's first non-empty prose or reasoning reached the
    /// city, read from the turn's clock (kernel-SPEC 8-75). A moment,
    /// not a duration: the time to first content is this minus the
    /// `model_called` line's `t`. Absent when the reply came through a
    /// door that reports nothing before it settles, when it streamed
    /// only tool calls, and on every line written before the key
    /// existed - none of which is a zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_at: Option<TimeMs>,
}

/// `steer_received`: text a person added at a phase boundary, and where
/// it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SteerReceived {
    pub source: String,
    pub text: String,
}

/// One part of a request's cache shape, as a record states it: how many
/// bytes the part carries, and their hash.
///
/// Both figures because two questions read this row. A prompt cache
/// compares the hash; a person asking why a request is expensive needs
/// the bytes, which no hash can answer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShapePart {
    pub bytes: u64,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,
}

/// Whether one region of a request differs from the request before it.
/// A closed pair rather than a bare `bool`, so a reader meets the word
/// that says which way it went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PartChange {
    Same,
    Moved,
}

/// What moved between one request's cache shape and the previous one.
///
/// `FirstRequest` is a variant rather than a silent "nothing moved": a
/// run's first request has no earlier shape to compare against, and
/// recording it as unchanged would read as a cache hit about to happen.
/// A miss is explained by naming what moved, which is what the second
/// variant is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "basis", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ShapeChanged {
    FirstRequest,
    Compared {
        /// The frozen segments whose bytes differ, by slot name, in
        /// prefix order. Empty means the system half is unchanged.
        system: Vec<String>,
        tools: PartChange,
        run: PartChange,
    },
}

/// `prompt_shape_compared`: one request's cache shape where a prompt
/// cache looks at it, and which of its regions moved since the request
/// before it in the same run.
///
/// The four segments are deliberately absent: `prompt_assembled`,
/// written immediately before this line, already states each one's hash
/// and length, and a second copy here would be a second home for them.
/// What this adds are the two regions that record never names - the tool
/// table and the conversation - and the comparison itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptShapeCompared {
    pub tools: ShapePart,
    pub run: ShapePart,
    /// The most bytes this request can be billed as input, counted from
    /// the parts above and the segments `prompt_assembled` states, added
    /// with checked arithmetic.
    pub upper_bound: u64,
    pub changed: ShapeChanged,
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
