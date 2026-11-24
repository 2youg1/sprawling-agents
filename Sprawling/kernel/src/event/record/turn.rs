// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::budget::UsdMicros;
use crate::event::identity::TimeMs;
use crate::event::payload::Payload;
use crate::locator::B3Hash;
use crate::model::{ModelUsage, StopReason};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptAssembled {
    #[serde(default)]
    pub segments: Vec<PromptSegment>,

    #[serde(default)]
    pub breakpoints: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<PromptTools>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptTools {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,

    pub len: u64,

    pub index: u64,

    #[serde(default)]
    pub reading: u64,

    #[serde(default)]
    pub always: u32,
    #[serde(default)]
    pub dormant: u32,
    #[serde(default)]
    pub skills: u32,

    pub rows: Vec<PromptTool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptTool {
    pub name: String,

    pub namespace: String,
    pub exposure: crate::config::Exposure,
    pub from: crate::config::ExposureFrom,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub digest: Option<crate::B3Hash>,

    pub len: u64,

    pub hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptSegment {
    pub slot: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,

    pub len: u64,

    #[serde(default)]
    pub sources: Vec<PromptSource>,

    #[serde(default)]
    pub skipped: Vec<PromptSkip>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptSource {
    pub addr: Address,
    pub kept: u64,
    pub marker: bool,
    pub dropped: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<B3Hash>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptSkip {
    pub addr: Address,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkipReason {
    Duplicate,

    Unreadable,

    NotUtf8,

    NoBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelCalled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_account: Option<super::ProviderAccountBinding>,

    #[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
    #[serde(default)]
    pub segments: Vec<B3Hash>,

    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelReturned {
    pub message: Payload,

    pub calls: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<std::collections::BTreeMap<String, serde_json::Value>>")
    )]
    pub usage: Option<ModelUsage>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub stop: Option<StopReason>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billed_usd_micros: Option<UsdMicros>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_at: Option<TimeMs>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_us: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_us: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SteerReceived {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<crate::ImageRef>,

    pub source: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idem: Option<crate::IdemKey>,
}

impl SteerReceived {
    pub const PERSON_SOURCE: &'static str = "user";

    pub const CITY_SOURCE: &'static str = "city";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShapePart {
    pub bytes: u64,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PartChange {
    Same,
    Moved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "basis", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ShapeChanged {
    FirstRequest,
    Compared {
        system: Vec<String>,
        tools: PartChange,
        run: PartChange,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptShapeCompared {
    pub tools: ShapePart,
    pub run: ShapePart,

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
