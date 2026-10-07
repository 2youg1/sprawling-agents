// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a privacy control writes, and what a read of it returns
//! (`crates/sprawling/spec/Privacy/Controls.lean`).
//!
//! A target names a place on the host; its operation kind and its scope
//! follow from the place and are not stored beside it (Privacy D68).

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};
use wire::{PrivacyScope, PrivacyTarget, PrivacyValue};

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
    /// Whose settings a write of this kind changes. Machine scope is
    /// written through an elevated child, which needs administrator
    /// approval (Privacy.Windows D60).
    pub(crate) const fn scope(self) -> PrivacyScope {
        match self {
            Self::RegistryValueHklm | Self::ScheduledTaskEnabled => PrivacyScope::Machine,
            Self::RegistryValueHkcu | Self::EnvironmentVariableUser => PrivacyScope::User,
        }
    }
}

impl From<&Target> for PrivacyTarget {
    fn from(target: &Target) -> Self {
        let owned = |text: &str| text.to_owned();
        match *target {
            Target::Registry {
                hive: Hive::LocalMachine,
                path,
                name,
            } => Self::RegistryValueHklm {
                path: owned(path),
                name: owned(name),
            },
            Target::Registry {
                hive: Hive::CurrentUser,
                path,
                name,
            } => Self::RegistryValueHkcu {
                path: owned(path),
                name: owned(name),
            },
            Target::UserEnvironment { name } => Self::EnvironmentVariableUser { name: owned(name) },
            Target::ScheduledTask { path, name } => Self::ScheduledTaskEnabled {
                path: owned(path),
                name: owned(name),
            },
        }
    }
}

/// A registry value exactly as the host holds it: absent, or a type code
/// and the raw bytes. Nothing is expanded, trimmed or decoded, so a value
/// written back is the value that was read (Privacy.State D61: whether the
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
/// and never compared as part of it (Privacy.State D61).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    pub(crate) value: Snapshot,
    pub(crate) key_existed: bool,
}

/// The registry type codes the controls write and the wire spells.
const REG_SZ: u32 = 1;
const REG_DWORD: u32 = 4;

impl RawValue {
    /// A `REG_DWORD`.
    pub(crate) fn dword(number: u32) -> Self {
        Self::Present {
            kind: REG_DWORD,
            bytes: number.to_le_bytes().to_vec(),
        }
    }

    /// A `REG_SZ`: `text` in UTF-16 with its terminating NUL.
    pub(crate) fn text(text: &str) -> Self {
        Self::Present {
            kind: REG_SZ,
            bytes: text
                .encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect(),
        }
    }
}

impl From<&Snapshot> for PrivacyValue {
    /// The one spelling of `snapshot` on the wire (wire D49).
    fn from(snapshot: &Snapshot) -> Self {
        match snapshot {
            Snapshot::Registry(RawValue::Absent) => Self::Absent,
            Snapshot::Registry(RawValue::Present { kind, bytes }) => present(*kind, bytes),
            Snapshot::Task(TaskState::Absent) => Self::TaskAbsent,
            Snapshot::Task(TaskState::Enabled { definition_sha256 }) => Self::TaskEnabled {
                definition_sha256: hex(definition_sha256),
            },
            Snapshot::Task(TaskState::Disabled { definition_sha256 }) => Self::TaskDisabled {
                definition_sha256: hex(definition_sha256),
            },
        }
    }
}

fn present(kind: u32, bytes: &[u8]) -> PrivacyValue {
    let dword = <[u8; 4]>::try_from(bytes)
        .ok()
        .filter(|_| kind == REG_DWORD);
    match (dword, text(kind, bytes)) {
        (Some(number), _) => PrivacyValue::Dword {
            number: u32::from_le_bytes(number),
        },
        (None, Some(text)) => PrivacyValue::Text { text },
        (None, None) => PrivacyValue::Raw {
            kind,
            hex: hex(bytes),
        },
    }
}

/// The text a `REG_SZ` holds when its bytes are UTF-16 ending in exactly
/// one terminating NUL; any other bytes have no text spelling.
fn text(kind: u32, bytes: &[u8]) -> Option<String> {
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|pair| <[u8; 2]>::try_from(pair).ok().map(u16::from_le_bytes))
        .collect::<Option<_>>()?;
    let (&last, body) = units.split_last()?;
    (kind == REG_SZ && last == 0)
        .then(|| {
            char::decode_utf16(body.iter().copied())
                .collect::<Result<String, _>>()
                .ok()
        })
        .flatten()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn bytes_of_hex(hex: &str) -> Option<Vec<u8>> {
    let digit = |byte: u8| match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|value| value.checked_add(10)),
        _ => None,
    };
    hex.as_bytes()
        .chunks(2)
        .map(|pair| match *pair {
            [high, low] => digit(high)?.checked_mul(16)?.checked_add(digit(low)?),
            _ => None,
        })
        .collect()
}

impl TryFrom<&PrivacyValue> for Snapshot {
    type Error = AxError;

    /// The snapshot `value` spells, refused unless `value` is that
    /// snapshot's one spelling (wire D49), so two spellings of one value
    /// never reach a comparison.
    fn try_from(value: &PrivacyValue) -> Result<Self, AxError> {
        let digest =
            |hex: &str| bytes_of_hex(hex).and_then(|bytes| <[u8; 32]>::try_from(bytes).ok());
        let snapshot = match value {
            PrivacyValue::Absent => Some(Self::Registry(RawValue::Absent)),
            PrivacyValue::Dword { number } => Some(Self::Registry(RawValue::dword(*number))),
            PrivacyValue::Text { text } => Some(Self::Registry(RawValue::text(text))),
            PrivacyValue::Raw { kind, hex } => bytes_of_hex(hex)
                .map(|bytes| Self::Registry(RawValue::Present { kind: *kind, bytes })),
            PrivacyValue::TaskAbsent => Some(Self::Task(TaskState::Absent)),
            PrivacyValue::TaskEnabled { definition_sha256 } => digest(definition_sha256)
                .map(|definition_sha256| Self::Task(TaskState::Enabled { definition_sha256 })),
            PrivacyValue::TaskDisabled { definition_sha256 } => digest(definition_sha256)
                .map(|definition_sha256| Self::Task(TaskState::Disabled { definition_sha256 })),
        };
        snapshot
            .filter(|snapshot| PrivacyValue::from(snapshot) == *value)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read the value a privacy page confirmed",
                    "the value is not spelled the way the host spells it",
                )
                .with_recovery("send back the value the privacy page was answered with, unchanged")
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn snapshots() -> impl Strategy<Value = Snapshot> {
        let bytes = proptest::collection::vec(any::<u8>(), 0..16);
        let text = ".{0,6}".prop_map(|text: String| Snapshot::Registry(RawValue::text(&text)));
        let digest = any::<[u8; 32]>();
        prop_oneof![
            Just(Snapshot::Registry(RawValue::Absent)),
            (0u32..12, bytes)
                .prop_map(|(kind, bytes)| Snapshot::Registry(RawValue::Present { kind, bytes })),
            any::<u32>().prop_map(|number| Snapshot::Registry(RawValue::dword(number))),
            text,
            Just(Snapshot::Task(TaskState::Absent)),
            digest.prop_map(|definition_sha256| Snapshot::Task(TaskState::Enabled {
                definition_sha256
            })),
            digest.prop_map(|definition_sha256| Snapshot::Task(TaskState::Disabled {
                definition_sha256
            })),
        ]
    }

    proptest! {
        /// Every snapshot has one wire spelling and comes back from it
        /// byte for byte (wire D49).
        #[test]
        fn a_snapshot_survives_its_wire_spelling(snapshot in snapshots()) {
            let value = PrivacyValue::from(&snapshot);
            prop_assert_eq!(Snapshot::try_from(&value).unwrap(), snapshot);
        }
    }

    /// A value spelled any other way than the host spells it is refused,
    /// so the comparison with a fresh read never meets two spellings.
    #[test]
    fn only_the_hosts_spelling_is_accepted() {
        let raw = |kind, hex: &str| PrivacyValue::Raw {
            kind,
            hex: hex.to_owned(),
        };
        for value in [
            raw(REG_DWORD, "01000000"),
            raw(REG_SZ, "31000000"),
            raw(3, "0g"),
            raw(3, "0"),
            raw(3, "AB"),
            PrivacyValue::TaskEnabled {
                definition_sha256: "00".repeat(31),
            },
        ] {
            assert_eq!(
                *Snapshot::try_from(&value).unwrap_err().code(),
                AxCode::InvalidArgs,
                "{value:?}"
            );
        }
    }
}
