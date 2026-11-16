// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorAnswer {
    pub items: Vec<DoctorItem>,

    pub tiers: Vec<DoctorVerdict>,

    pub sandbox: DoctorSandbox,

    pub custody: DoctorCustody,

    pub core: DoctorCore,

    pub scanning: super::scanning::DoctorScanning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCore {
    Raised,

    HeldBySetting,

    Refused { said: String },

    LoweredByValve,

    Unasked { said: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorTier {
    Use,

    Develop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorNeed {
    Required,

    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorItem {
    pub name: String,
    pub tier: DoctorTier,
    pub need: DoctorNeed,

    pub homepage: Option<String>,
    pub state: DoctorState,
    pub install: DoctorInstall,

    pub pinned: Option<String>,

    pub pack: Option<DoctorPack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorPack {
    RustTools,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorUpstream {
    pub item: String,
    pub newest: DoctorNewest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorNewest {
    Asking,

    Read { version: String },

    Unread { why: DoctorUnread },

    Refused { said: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorUnread {
    WithToolchain,

    ManyBrands,

    MatchesBrowser,

    ThisProject,

    NoSource,

    UnknownItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorState {
    Present { at: String, version: DoctorVersion },

    Broken { at: String, fault: DoctorFault },
    Absent { absence: DoctorAbsence },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorVersion {
    Said { text: String },

    Silent,

    Unreadable,

    Late,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorFault {
    WillNotStart { said: String },

    HalfWritten,
    Unreadable { said: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorAbsence {
    NotOnSearchPath,

    VariableNamesNothing { variable: String, path: String },
    NoComponent { dir: String },
    NoHome,
    NotInThisBuild,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorInstall {
    Command { spelled: String },

    Print { spelled: String },

    Manual { how: String },

    UnknownPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorVerdict {
    pub tier: DoctorTier,

    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorSandbox {
    pub arm: DoctorSandboxArm,

    pub named: SandboxArm,

    pub coverage: Vec<DoctorGuarantee>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorSandboxArm {
    LinuxNamespaces,

    WindowsJobObject,

    MacosSeatbelt,

    CopiedTree,

    Unavailable { missing: DoctorSandboxMissing },
}

pub use kernel::SandboxArm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorSandboxMissing {
    ScratchDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorGuarantee {
    pub axis: DoctorGuaranteeAxis,
    pub kept: DoctorCoverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorGuaranteeAxis {
    Filesystem,
    Network,
    ProcessTree,
    User,
    Resources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCoverage {
    Kept,
    NotKept,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorCustody {
    pub store: DoctorCustodyStore,
    pub keeps: DoctorCustodyLifetime,

    pub refusal: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCustodyStore {
    PlatformService,

    EncryptedFile,

    SessionMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCustodyLifetime {
    AcrossReboots,
    WithPassphrase,
    UntilReboot,
    ThisProcess,
}
