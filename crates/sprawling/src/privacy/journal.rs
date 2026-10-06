// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Strict read-only local JSONL (`crates/sprawling/spec/Privacy/Journal.lean`).

use std::fs::{File, TryLockError};
use std::io::{ErrorKind, Read};
use std::path::Path;

use super::state::{History, HistoryFault, Line};

/// History input bound, not a claim about Windows registry value limits.
const HISTORY_BYTES_MAX: u64 = 8 * 1024 * 1024;

pub(super) fn read(path: &Path) -> Result<History, HistoryFault> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(History::default()),
        Err(source) => return Err(HistoryFault::Io(source)),
    };
    match file.try_lock_shared() {
        Ok(()) => (),
        Err(TryLockError::WouldBlock) => return Err(HistoryFault::Busy),
        Err(TryLockError::Error(source)) => return Err(HistoryFault::Io(source)),
    }
    let mut bytes = Vec::new();
    (&file)
        .take(HISTORY_BYTES_MAX.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(HistoryFault::Io)?;
    if u64::try_from(bytes.len())
        .map_err(|source| HistoryFault::Io(std::io::Error::other(source)))?
        > HISTORY_BYTES_MAX
    {
        return Err(HistoryFault::Capacity);
    }
    let history = decode(&bytes)?;
    drop(file);
    Ok(history)
}

fn decode(bytes: &[u8]) -> Result<History, HistoryFault> {
    if bytes.is_empty() {
        return Ok(History::default());
    }
    if bytes.last() != Some(&b'\n') {
        return Err(HistoryFault::Invalid("history ends in an incomplete line"));
    }
    let mut lines = Vec::new();
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        lines.push(serde_json::from_slice::<Line>(line).map_err(HistoryFault::Decode)?);
    }
    History::fold(lines)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::super::state::{Control, DEFINITION, Event, Intent, Outcome, RawValue, SCHEMA};
    use super::*;
    use std::num::NonZeroU64;

    fn intent(operation: u64, original: RawValue) -> Intent {
        let modified = RawValue::Present {
            kind: 1,
            bytes: vec![49, 0, 0, 0],
        };
        Intent {
            operation: NonZeroU64::new(operation).unwrap(),
            control: Control::WindowsUserPowershellTelemetry,
            definition: DEFINITION,
            owner: "fixture-owner".to_owned(),
            original,
            modified: modified.clone(),
            recommendation: modified,
            restore_of: None,
        }
    }

    fn prepared(intent: Intent) -> Line {
        Line {
            schema: SCHEMA,
            event: Event::Prepared { intent },
        }
    }

    fn finished(operation: u64, outcome: Outcome) -> Line {
        Line {
            schema: SCHEMA,
            event: Event::Finished {
                operation: NonZeroU64::new(operation).unwrap(),
                outcome,
            },
        }
    }

    fn bytes(lines: &[Line]) -> Vec<u8> {
        lines
            .iter()
            .flat_map(|line| {
                let mut bytes = serde_json::to_vec(line).unwrap();
                bytes.push(b'\n');
                bytes
            })
            .collect()
    }

    #[test]
    fn reading_missing_history_creates_nothing() {
        let home = tempfile::tempdir().unwrap();
        let path = accounting::home::Home::at(home.path()).privacy_history();
        assert!(read(&path).unwrap().statuses().is_empty());
        assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
    }

    #[test]
    fn reading_unfinished_history_never_settles_or_repairs_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let before = bytes(&[prepared(intent(1, RawValue::Absent { key_existed: false }))]);
        std::fs::write(&path, &before).unwrap();
        let status = serde_json::to_value(read(&path).unwrap().statuses()).unwrap();
        assert_eq!(
            status,
            serde_json::json!([{ "operation": 1, "outcome": "unresolved" }])
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut torn = before.clone();
        torn.extend_from_slice(b"{\"schema\":");
        std::fs::write(&path, &torn).unwrap();
        assert!(matches!(read(&path), Err(HistoryFault::Invalid(_))));
        assert_eq!(std::fs::read(&path).unwrap(), torn);
    }

    #[test]
    fn history_read_refuses_an_exclusive_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let writer = File::create(&path).unwrap();
        writer.try_lock().unwrap();
        assert!(matches!(read(&path), Err(HistoryFault::Busy)));
        drop(writer);
        assert!(read(&path).unwrap().statuses().is_empty());
    }

    #[test]
    fn unknown_or_malformed_history_is_not_empty_success() {
        for input in [
            b"\n".as_slice(),
            b"{}\n",
            b"{\"schema\":999,\"event\":{\"phase\":\"future\"}}\n",
            b"\xff\n",
        ] {
            assert!(decode(input).is_err());
        }
        assert!(decode(&bytes(&[finished(1, Outcome::Applied)])).is_err());
        let change = intent(1, RawValue::Absent { key_existed: true });
        assert!(
            decode(&bytes(&[
                prepared(change.clone()),
                finished(1, Outcome::Unknown),
                prepared(change)
            ]))
            .is_err()
        );
    }

    /// Derived from Privacy.restoreVectors: only the latest owned operation may reverse.
    #[test]
    fn restore_vectors_preserve_absence_type_bytes_and_latest_ownership() {
        for original in [
            RawValue::Absent { key_existed: false },
            RawValue::Present {
                kind: 1,
                bytes: vec![],
            },
            RawValue::Present {
                kind: 2,
                bytes: vec![37, 0, 0, 0],
            },
        ] {
            let first = intent(1, original);
            let second = intent(
                2,
                RawValue::Present {
                    kind: 1,
                    bytes: vec![48, 0, 0, 0],
                },
            );
            for old in [&first, &second] {
                let mut restore = intent(3, old.modified.clone());
                restore.modified = old.original.clone();
                restore.restore_of = Some(old.operation);
                let history = bytes(&[
                    prepared(first.clone()),
                    finished(1, Outcome::Applied),
                    prepared(second.clone()),
                    finished(2, Outcome::Applied),
                    prepared(restore),
                    finished(3, Outcome::Restored),
                ]);
                assert_eq!(decode(&history).is_ok(), old.operation == second.operation);
                let decoded: Vec<Line> = history
                    .split_inclusive(|b| *b == b'\n')
                    .map(|line| serde_json::from_slice(line).unwrap())
                    .collect();
                assert_eq!(bytes(&decoded), history);
            }
        }
    }
    #[test]
    fn unknown_fields_duplicate_ids_and_cross_identity_history_are_refused() {
        let first = intent(1, RawValue::Absent { key_existed: true });
        let mut other = intent(2, first.original.clone());
        other.owner = "another-fixture-owner".to_owned();
        assert!(
            decode(&bytes(&[
                prepared(first.clone()),
                finished(1, Outcome::Applied),
                prepared(other)
            ]))
            .is_err()
        );
        assert!(
            decode(&bytes(&[
                prepared(first.clone()),
                finished(1, Outcome::Applied),
                prepared(first.clone())
            ]))
            .is_err()
        );
        let mut value = serde_json::to_value(prepared(first)).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_owned(), serde_json::json!(true));
        let mut line = serde_json::to_vec(&value).unwrap();
        line.push(b'\n');
        assert!(decode(&line).is_err());
    }

    #[test]
    fn plaintext_identity_and_old_schema_are_refused_without_rewriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let prepared = prepared(intent(1, RawValue::Absent { key_existed: true }));
        for (schema, owner) in [
            (SCHEMA, "fixture-private-principal"),
            (1, "secret:privacy/fixture-owner"),
        ] {
            let mut line = serde_json::to_value(&prepared).unwrap();
            line["schema"] = serde_json::json!(schema);
            line["event"]["intent"]["owner"] = serde_json::json!(owner);
            let mut before = serde_json::to_vec(&line).unwrap();
            before.push(b'\n');
            std::fs::write(&path, &before).unwrap();
            assert!(read(&path).is_err(), "unsafe history was accepted");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn decoding_refusal_never_repeats_private_input() {
        let prepared = prepared(intent(1, RawValue::Absent { key_existed: true }));
        for field in ["control", "unexpected", "owner"] {
            let mut line = serde_json::to_value(&prepared).unwrap();
            if field == "unexpected" {
                line["event"]["intent"]["fixture-private-principal"] = serde_json::json!(true);
            } else {
                line["event"]["intent"][field] = serde_json::json!("fixture-private-principal");
            }
            let mut before = serde_json::to_vec(&line).unwrap();
            before.push(b'\n');
            let fault = match decode(&before) {
                Ok(_) => panic!("private input accepted"),
                Err(fault) => fault,
            };
            assert!(!format!("{fault:?}").contains("fixture-private-principal"));
            let fault = fault.into_ax();
            assert!(!fault.to_string().contains("fixture-private-principal"));
            assert_eq!(fault.code(), &kernel::AxCode::StorageFatal);
            assert!(fault.subject().contains("column"));
        }
    }

    proptest::proptest! {
        #[test]
        fn original_raw_values_roundtrip_without_normalization(
            kind in proptest::num::u32::ANY,
            raw in proptest::collection::vec(proptest::num::u8::ANY, 0..512),
            key_existed in proptest::bool::ANY,
        ) {
            for original in [RawValue::Absent { key_existed }, RawValue::Present { kind, bytes: raw }] {
                let before = intent(1, original);
                let encoded = serde_json::to_vec(&before).unwrap();
                let decoded: Intent = serde_json::from_slice(&encoded).unwrap();
                proptest::prop_assert!(before == decoded);
            }
        }
    }
}
