// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The answer to `Query::Privacy` (`crates/wire/spec/Privacy.lean` §8-88):
//! the host, every control with its current value, every original item
//! that is not written, what this app's history discloses, and the
//! results the host keeps for the operations pages sent.

use kernel::AxError;
use serde::{Deserialize, Serialize};

use super::operation::PrivacyOutcome;
use super::{
    PrivacyBuildEffect, PrivacyCategory, PrivacyControl, PrivacyEdition, PrivacyEditionFit,
    PrivacyNotWritten, PrivacyOriginal, PrivacyScope,
};

/// Everything the privacy page shows, read in one pass on the host that
/// runs the city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyAnswer {
    pub host: PrivacyHost,
    /// Every control, in page order.
    pub controls: Vec<PrivacyControlEntry>,
    /// Every original item that is not written, in list order.
    pub not_written: Vec<PrivacyNotWrittenEntry>,
    pub history: PrivacyHistory,
    /// The results the host still keeps, oldest first; a page reads the
    /// ones whose `idem` it minted.
    pub outcomes: Vec<PrivacyOutcome>,
}

/// The machine the controls belong to: the one running this city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyHost {
    Windows(PrivacyWindows),
    /// A Windows host whose version record could not be read; every
    /// control's edition fit is then `not_stated`.
    Unreadable {
        error: AxError,
    },
    /// The controls exist only on Windows; nothing is read or written.
    NotWindows,
}

/// What `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyWindows {
    /// `EditionID` as written, for example `CoreCountrySpecific`.
    pub edition_id: String,
    /// The edition `edition_id` names; absent when it names none of the
    /// editions the controls table lists.
    pub edition: Option<PrivacyEdition>,
    /// `CurrentBuild`, then `.UBR` when the update revision is recorded.
    pub build: String,
    /// `DisplayVersion`, for example `24H2`; builds before 20H2 have none.
    pub display_version: Option<String>,
}

/// One control as the host's table defines it and as the host reads now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyControlEntry {
    pub control: PrivacyControl,
    pub category: PrivacyCategory,
    /// The original items this control writes, in list order; empty for a
    /// control added beyond the original list.
    pub originals: Vec<PrivacyOriginalLine>,
    pub target: PrivacyTarget,
    /// `machine` is written through administrator approval.
    pub scope: PrivacyScope,
    pub editions: PrivacyEditions,
    /// How `editions` reads for this host's edition.
    pub host_fit: PrivacyEditionFit,
    pub build_effect: PrivacyBuildEffect,
    pub current: PrivacyCurrent,
    /// What an apply leaves over `current`; absent when `current` was not
    /// read, or when the host has no task to disable.
    pub written: Option<PrivacyValue>,
}

/// Where Microsoft's documentation says a control is honoured and where it
/// is ignored; an edition in neither list is not stated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyEditions {
    pub honoured: Vec<PrivacyEdition>,
    pub ignored: Vec<PrivacyEdition>,
}

/// One line of the request list, as the person wrote it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyOriginalLine {
    pub item: PrivacyOriginal,
    pub text: String,
}

/// An original item no control writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyNotWrittenEntry {
    pub line: PrivacyOriginalLine,
    pub reason: PrivacyNotWritten,
    /// The controls that come closest, in page order; possibly none.
    pub alternatives: Vec<PrivacyControl>,
}

/// The place on the host a control writes, named by its operation kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyTarget {
    /// The value `name` under `HKLM\<path>`, in the 64-bit view.
    RegistryValueHklm { path: String, name: String },
    /// The value `name` under `HKCU\<path>`.
    RegistryValueHkcu { path: String, name: String },
    /// The variable `name` of the user's persistent environment.
    EnvironmentVariableUser { name: String },
    /// The task `name` in the task folder `path`.
    ScheduledTaskEnabled { path: String, name: String },
}

/// What a read of a control's target gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyCurrent {
    Read {
        value: PrivacyValue,
    },
    /// This account may not read the target.
    AccessDenied,
    Failed {
        error: AxError,
    },
    /// The host is not Windows, so nothing was read.
    NotRead,
}

/// One value of a target, exactly as the host holds it.
///
/// Every value has exactly one spelling: a registry value is `dword` when
/// it is a `REG_DWORD` of four bytes, `text` when it is a `REG_SZ` whose
/// bytes are UTF-16 ending in one terminating NUL, and `raw` otherwise. A
/// page sends a value back as `expected` unchanged, and the host turns it
/// into the bytes it compares with what it reads, refusing any other
/// spelling (`crates/wire/spec/Privacy.lean` D50).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyValue {
    /// The registry value does not exist.
    Absent,
    Dword {
        number: u32,
    },
    Text {
        text: String,
    },
    /// Any other registry value: its type code and its bytes in lowercase
    /// hex.
    Raw {
        kind: u32,
        hex: String,
    },
    /// The host has no such task.
    TaskAbsent,
    /// The digest covers the task's definition without its enabled flag,
    /// in lowercase hex.
    TaskEnabled {
        definition_sha256: String,
    },
    TaskDisabled {
        definition_sha256: String,
    },
}

/// What this app's history tells the page, which depends on who asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyHistory {
    /// The account running the city is the one the history belongs to,
    /// or the history is empty and has nobody to belong to.
    Disclosed {
        /// For each control this app still owns a change of, the latest
        /// such change, in page order.
        owned: Vec<PrivacyIntent>,
        /// The operation that has no conclusion; every write waits for
        /// the person's check of it.
        unresolved: Option<PrivacyIntent>,
    },
    /// The identity check refused or could not be made; no recorded
    /// value is shown.
    Withheld { error: AxError },
    /// The history could not be read: another operation holds it, or it
    /// is damaged.
    Unreadable { error: AxError },
}

/// One recorded operation: the value it found and the value it wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyIntent {
    /// Never zero.
    pub operation: u64,
    pub control: PrivacyControl,
    pub original: PrivacyValue,
    pub modified: PrivacyValue,
    /// The operation a restore undoes; absent for an apply.
    pub restore_of: Option<u64>,
}
