// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a privacy control writes, and what a read of it returns
//! (`crates/sprawling/spec/Privacy/Controls.lean`).
//!
//! A target names a place on the host; its operation kind and its scope
//! follow from the place and are not stored beside it (Privacy D65).

use serde::{Deserialize, Serialize};

/// Which registry hive a registry target lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hive {
    CurrentUser,
    LocalMachine,
}

/// One place on the host a control writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// A value under `path` in `hive`, read and written in the 64-bit view.
    Registry {
        hive: Hive,
        path: &'static str,
        name: &'static str,
    },
    /// A variable of the user's persistent environment: the value `name`
    /// under `HKCU\Environment`, announced to running programs after a write.
    UserEnvironment { name: &'static str },
    /// The enabled state of the task `name` in the task folder `path`.
    ScheduledTask {
        path: &'static str,
        name: &'static str,
    },
}

/// How a control is written: the closed set of four operation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationKind {
    RegistryValueHklm,
    RegistryValueHkcu,
    EnvironmentVariableUser,
    ScheduledTaskEnabled,
}

/// Whose settings a write changes. Machine scope is written through an
/// elevated child, which needs administrator approval (Privacy.Windows D57).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    dead_code,
    reason = "the privacy answer and the coordinator are its readers; until they exist only tests and the compile-time tallies read it"
)]
pub(crate) enum Scope {
    User,
    Machine,
}

impl Target {
    pub(crate) const fn kind(&self) -> OperationKind {
        match self {
            Self::Registry {
                hive: Hive::LocalMachine,
                ..
            } => OperationKind::RegistryValueHklm,
            Self::Registry {
                hive: Hive::CurrentUser,
                ..
            } => OperationKind::RegistryValueHkcu,
            Self::UserEnvironment { .. } => OperationKind::EnvironmentVariableUser,
            Self::ScheduledTask { .. } => OperationKind::ScheduledTaskEnabled,
        }
    }
}

impl OperationKind {
    #[expect(
        dead_code,
        reason = "the privacy answer and the coordinator are its readers; until they exist only tests and the compile-time tallies read it"
    )]
    pub(crate) const fn scope(self) -> Scope {
        match self {
            Self::RegistryValueHklm | Self::ScheduledTaskEnabled => Scope::Machine,
            Self::RegistryValueHkcu | Self::EnvironmentVariableUser => Scope::User,
        }
    }
}

/// A registry value exactly as the host holds it: absent, or a type code
/// and the raw bytes. Nothing is expanded, trimmed or decoded, so a value
/// written back is the value that was read (Privacy.State D58: whether the
/// parent key existed is recorded beside the value, not inside it).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RawValue {
    Absent,
    Present { kind: u32, bytes: Vec<u8> },
}

/// A scheduled task as the host holds it. The digest covers the task's
/// definition with its enabled flag removed, so disabling a task leaves
/// its digest unchanged and any other edit to the task changes it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskState {
    Absent,
    Enabled { definition_sha256: [u8; 32] },
    Disabled { definition_sha256: [u8; 32] },
}

/// One read of a control's target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Snapshot {
    Registry(RawValue),
    Task(TaskState),
}

/// One successful read of a control's target: its snapshot, and whether a
/// registry value's parent key exists, which is recorded beside the value
/// and never compared as part of it (Privacy.State D58).
#[cfg_attr(
    not(any(test, windows)),
    expect(dead_code, reason = "read only by the coordinator, not yet called")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    pub(crate) value: Snapshot,
    pub(crate) key_existed: bool,
}
