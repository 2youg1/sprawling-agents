// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! History lines for the tests of this module and of its readers.

use super::*;
use crate::privacy::target::RawValue;

pub(in crate::privacy) fn owner() -> SecretRef {
    SecretRef::new("privacy", "fixture-owner").unwrap()
}

pub(in crate::privacy) fn dword(value: u32) -> Snapshot {
    Snapshot::Registry(RawValue::Present {
        kind: 4,
        bytes: value.to_le_bytes().to_vec(),
    })
}

pub(in crate::privacy) fn operation(id: u64) -> NonZeroU64 {
    NonZeroU64::new(id).unwrap()
}

pub(in crate::privacy) fn apply(
    id: u64,
    control: PrivacyControl,
    original: Snapshot,
    modified: Snapshot,
) -> Intent {
    Intent {
        operation: operation(id),
        control,
        owner: owner(),
        original,
        modified,
        key_existed: true,
        restore_of: None,
    }
}

/// The restore that reverses `owned`.
pub(in crate::privacy) fn restore(id: u64, owned: &Intent) -> Intent {
    Intent {
        operation: operation(id),
        original: owned.modified.clone(),
        modified: owned.original.clone(),
        restore_of: Some(owned.operation),
        ..owned.clone()
    }
}

pub(in crate::privacy) fn prepared(intent: &Intent) -> Line {
    Line {
        schema: SCHEMA,
        event: Event::Prepared {
            intent: intent.clone(),
        },
    }
}

pub(in crate::privacy) fn finished(intent: &Intent, outcome: Outcome) -> Line {
    Line {
        schema: SCHEMA,
        event: Event::Finished {
            operation: intent.operation,
            outcome,
        },
    }
}

pub(in crate::privacy) fn reconciled(intent: &Intent, settlement: PrivacySettlement) -> Line {
    Line {
        schema: SCHEMA,
        event: Event::Reconciled {
            operation: intent.operation,
            settlement,
        },
    }
}

pub(in crate::privacy) fn bytes(lines: &[Line]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|line| {
            let mut bytes = serde_json::to_vec(line).unwrap();
            bytes.push(b'\n');
            bytes
        })
        .collect()
}
