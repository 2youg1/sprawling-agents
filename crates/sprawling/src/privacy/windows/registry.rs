// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Exact raw registry values under HKCU and HKLM
//! (`crates/sprawling/spec/Privacy/Windows.lean`).
//!
//! A value is read and written as its type code and its bytes, never
//! decoded, expanded or trimmed, so what a restore writes back is what
//! the apply read. Every key is opened through the 64-bit view, which is
//! where the policy templates and Settings write. The adapter judges
//! nothing about the path it is given: which targets exist is decided by
//! the control table alone.

use std::io::ErrorKind;

use winreg::enums::{
    KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WOW64_64KEY, REG_BINARY, REG_DWORD, REG_DWORD_BIG_ENDIAN,
    REG_EXPAND_SZ, REG_FULL_RESOURCE_DESCRIPTOR, REG_LINK, REG_MULTI_SZ, REG_NONE, REG_QWORD,
    REG_RESOURCE_LIST, REG_RESOURCE_REQUIREMENTS_LIST, REG_SZ, RegType,
};
use winreg::{HKEY, RegKey, RegValue};

use super::super::target::RawValue;

/// The type codes `RegQueryValueExW` reports, each with the `winreg`
/// type that writes it back; the one table both directions read.
const REGISTRY_TYPES: [(u32, RegType); 12] = [
    (0, REG_NONE),
    (1, REG_SZ),
    (2, REG_EXPAND_SZ),
    (3, REG_BINARY),
    (4, REG_DWORD),
    (5, REG_DWORD_BIG_ENDIAN),
    (6, REG_LINK),
    (7, REG_MULTI_SZ),
    (8, REG_RESOURCE_LIST),
    (9, REG_FULL_RESOURCE_DESCRIPTOR),
    (10, REG_RESOURCE_REQUIREMENTS_LIST),
    (11, REG_QWORD),
];

/// What one read of a value found.
pub(in crate::privacy) enum Reading {
    /// The value or its absence, and whether its key exists, which is
    /// recorded beside the value rather than inside it (Privacy.State D60).
    Value { value: RawValue, key_existed: bool },
    /// `ERROR_ACCESS_DENIED`: this account may not read the key.
    AccessDenied,
    /// Any other failure, kept so the caller can say what it was.
    Failed(std::io::Error),
}

/// Why a write did not reach the registry.
pub(in crate::privacy) enum WriteFault {
    /// `ERROR_ACCESS_DENIED`: this account may not write the key.
    AccessDenied,
    /// The value has no exact raw encoding (see [`rewritable`]); nothing
    /// was written.
    NotRewritable,
    /// Any other failure; the value may or may not have changed, which
    /// only a readback can tell.
    Failed(std::io::Error),
}

/// Reads `name` under `key` of `hive`.
pub(in crate::privacy) fn read(hive: HKEY, key: &str, name: &str) -> Reading {
    let opened =
        match RegKey::predef(hive).open_subkey_with_flags(key, KEY_QUERY_VALUE | KEY_WOW64_64KEY) {
            Ok(opened) => opened,
            Err(missing) if missing.kind() == ErrorKind::NotFound => {
                return Reading::Value {
                    value: RawValue::Absent,
                    key_existed: false,
                };
            }
            Err(other) => return unreadable(other),
        };
    match opened.get_raw_value(name) {
        Ok(value) => match code_of(&value.vtype) {
            Some(kind) => Reading::Value {
                value: RawValue::Present {
                    kind,
                    bytes: value.bytes.into_owned(),
                },
                key_existed: true,
            },
            None => Reading::Failed(std::io::Error::from(ErrorKind::InvalidData)),
        },
        Err(missing) if missing.kind() == ErrorKind::NotFound => Reading::Value {
            value: RawValue::Absent,
            key_existed: true,
        },
        Err(other) => unreadable(other),
    }
}

/// Makes `name` under `key` of `hive` equal `value`: a present value is
/// written under its own type code, creating the key when it is missing;
/// an absent one deletes only the value and keeps the key, so a key the
/// apply created stays behind as an empty key the report can name
/// (Privacy.State D60).
///
/// # Errors
/// [`WriteFault`]; a write that fails part-way is told apart by the
/// readback, not here.
pub(in crate::privacy) fn write(
    hive: HKEY,
    key: &str,
    name: &str,
    value: &RawValue,
) -> Result<(), WriteFault> {
    let root = RegKey::predef(hive);
    match value {
        RawValue::Absent => {
            match root.open_subkey_with_flags(key, KEY_SET_VALUE | KEY_WOW64_64KEY) {
                Ok(opened) => match opened.delete_value(name) {
                    Ok(()) => Ok(()),
                    Err(missing) if missing.kind() == ErrorKind::NotFound => Ok(()),
                    Err(other) => Err(unwritable(other)),
                },
                Err(missing) if missing.kind() == ErrorKind::NotFound => Ok(()),
                Err(other) => Err(unwritable(other)),
            }
        }
        RawValue::Present { kind, bytes } => {
            let vtype = type_of(*kind)
                .filter(|_| rewritable(value))
                .ok_or(WriteFault::NotRewritable)?;
            let (opened, _created) = root
                .create_subkey_with_flags(key, KEY_SET_VALUE | KEY_WOW64_64KEY)
                .map_err(unwritable)?;
            opened
                .set_raw_value(
                    name,
                    &RegValue {
                        vtype,
                        bytes: bytes.as_slice().into(),
                    },
                )
                .map_err(unwritable)
        }
    }
}

/// Whether `value` can be written back byte for byte. Absence always can;
/// a string type only when it is whole UTF-16 code units ending in a
/// terminator, because `RegSetValueExW` is documented only for strings
/// whose size includes the terminator, and an original without one could
/// come back changed. A type code outside [`REGISTRY_TYPES`] cannot.
pub(in crate::privacy) fn rewritable(value: &RawValue) -> bool {
    match value {
        RawValue::Absent => true,
        RawValue::Present { kind, bytes } => type_of(*kind).is_some_and(|vtype| {
            !matches!(vtype, REG_SZ | REG_EXPAND_SZ)
                || (bytes.len().is_multiple_of(2) && bytes.ends_with(&[0, 0]))
        }),
    }
}

fn code_of(vtype: &RegType) -> Option<u32> {
    REGISTRY_TYPES
        .iter()
        .find(|(_, known)| known == vtype)
        .map(|(code, _)| *code)
}

fn type_of(code: u32) -> Option<RegType> {
    REGISTRY_TYPES
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, vtype)| vtype.clone())
}

/// `ERROR_ACCESS_DENIED` is the one failure the page names; the standard
/// library reads it as `PermissionDenied`.
fn unreadable(failure: std::io::Error) -> Reading {
    if failure.kind() == ErrorKind::PermissionDenied {
        Reading::AccessDenied
    } else {
        Reading::Failed(failure)
    }
}

fn unwritable(failure: std::io::Error) -> WriteFault {
    if failure.kind() == ErrorKind::PermissionDenied {
        WriteFault::AccessDenied
    } else {
        WriteFault::Failed(failure)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    fn fresh_name() -> String {
        let mut suffix = [0_u8; 8];
        getrandom::fill(&mut suffix).unwrap();
        format!("sprawling-absent-{:016x}", u64::from_le_bytes(suffix))
    }

    fn value(reading: Reading) -> (RawValue, bool) {
        match reading {
            Reading::Value { value, key_existed } => (value, key_existed),
            Reading::AccessDenied => panic!("access denied"),
            Reading::Failed(failure) => panic!("{failure}"),
        }
    }

    /// Read only: a key named by a fresh random suffix is absent, and so
    /// is its key; nothing is created by asking.
    #[test]
    fn a_key_that_does_not_exist_reads_as_absent_without_its_key() {
        let key = format!(r"Software\{}", fresh_name());
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            assert_eq!(value(read(hive, &key, "Value")), (RawValue::Absent, false));
        }
        assert!(matches!(
            RegKey::predef(HKEY_CURRENT_USER).open_subkey(&key),
            Err(missing) if missing.kind() == ErrorKind::NotFound
        ));
    }

    /// Read only: a value missing from a key that exists keeps the fact
    /// that the key exists, and a recorded string comes back as its type
    /// code and its terminated UTF-16 bytes.
    #[test]
    fn an_existing_key_reads_its_values_raw() {
        let key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        assert_eq!(
            value(read(HKEY_LOCAL_MACHINE, key, &fresh_name())),
            (RawValue::Absent, true)
        );
        let (build, key_existed) = value(read(HKEY_LOCAL_MACHINE, key, "CurrentBuild"));
        assert!(key_existed);
        assert!(matches!(&build, RawValue::Present { kind: 1, .. }));
        assert!(rewritable(&build));
    }

    /// Every type code maps to one `winreg` type and back, so a value is
    /// written under exactly the code it was read with.
    #[test]
    fn the_type_table_round_trips() {
        for code in 0..=11 {
            let vtype = type_of(code).unwrap();
            assert_eq!(code_of(&vtype), Some(code));
        }
        assert!(type_of(12).is_none());
    }

    #[test]
    fn only_exactly_encodable_values_are_rewritable() {
        let present = |kind, bytes: &[u8]| RawValue::Present {
            kind,
            bytes: bytes.to_vec(),
        };
        for accepted in [
            RawValue::Absent,
            present(1, &[49, 0, 0, 0]),
            present(2, &[0, 0]),
            present(4, &[1, 0, 0, 0]),
            present(3, &[7]),
        ] {
            assert!(rewritable(&accepted), "{accepted:?}");
        }
        for refused in [
            present(1, &[49, 0]),
            present(1, &[49, 0, 0]),
            present(2, &[]),
            present(12, &[0, 0, 0, 0]),
        ] {
            assert!(!rewritable(&refused), "{refused:?}");
        }
    }
}
