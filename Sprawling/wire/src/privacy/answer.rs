// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use serde::{Deserialize, Serialize};

use super::operation::PrivacyOutcome;
use super::{
    PrivacyBuildEffect, PrivacyCategory, PrivacyControl, PrivacyEdition, PrivacyEditionFit,
    PrivacyNotWritten, PrivacyOriginal, PrivacyScope,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyAnswer {
    pub host: PrivacyHost,

    pub controls: Vec<PrivacyControlEntry>,

    pub not_written: Vec<PrivacyNotWrittenEntry>,
    pub history: PrivacyHistory,

    pub outcomes: Vec<PrivacyOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyHost {
    Windows(PrivacyWindows),

    Unreadable {
        error: AxError,
    },

    NotWindows,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyWindows {

    pub edition_id: String,

    pub edition: Option<PrivacyEdition>,

    pub build: String,

    pub display_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyControlEntry {
    pub control: PrivacyControl,
    pub category: PrivacyCategory,

    pub originals: Vec<PrivacyOriginalLine>,
    pub target: PrivacyTarget,

    pub scope: PrivacyScope,
    pub editions: PrivacyEditions,

    pub host_fit: PrivacyEditionFit,
    pub build_effect: PrivacyBuildEffect,
    pub current: PrivacyCurrent,

    pub written: Option<PrivacyValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyEditions {
    pub honoured: Vec<PrivacyEdition>,
    pub ignored: Vec<PrivacyEdition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyOriginalLine {
    pub item: PrivacyOriginal,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyNotWrittenEntry {
    pub line: PrivacyOriginalLine,
    pub reason: PrivacyNotWritten,

    pub alternatives: Vec<PrivacyControl>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyTarget {

    RegistryValueHklm { path: String, name: String },

    RegistryValueHkcu { path: String, name: String },

    EnvironmentVariableUser { name: String },

    ScheduledTaskEnabled { path: String, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyCurrent {
    Read {
        value: PrivacyValue,
    },

    AccessDenied,
    Failed {
        error: AxError,
    },

    NotRead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyValue {

    Absent,
    Dword {
        number: u32,
    },
    Text {
        text: String,
    },

    Raw {
        kind: u32,
        hex: String,
    },

    TaskAbsent,

    TaskEnabled {
        definition_sha256: String,
    },
    TaskDisabled {
        definition_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyHistory {

    Disclosed {

        owned: Vec<PrivacyIntent>,

        unresolved: Option<PrivacyIntent>,
    },

    Withheld { error: AxError },

    Unreadable { error: AxError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyIntent {

    pub operation: u64,
    pub control: PrivacyControl,
    pub original: PrivacyValue,
    pub modified: PrivacyValue,

    pub restore_of: Option<u64>,
}
