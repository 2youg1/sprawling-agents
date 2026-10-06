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
        lines.push(serde_json::from_slice::<Line>(line).map_err(|source| {
            HistoryFault::Decode {
                line: source.line(),
                column: source.column(),
                category: source.classify(),
            }
        })?);
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
    use super::super::state::fixtures::{apply, bytes, dword, finished, prepared};
    use super::super::state::{Outcome, SCHEMA};
    use super::super::target::{RawValue, Snapshot};
    use super::*;
    use wire::PrivacyControl;

    const PRIVATE_INPUT: &str = "fixture-private-principal";
    const CONTROL: PrivacyControl = PrivacyControl::PowershellTelemetryOptout;

    fn absent() -> Snapshot {
        Snapshot::Registry(RawValue::Absent)
    }

    #[test]
    fn reading_missing_history_creates_nothing() {
        let home = tempfile::tempdir().unwrap();
        let path = accounting::home::Home::at(home.path()).privacy_history();
        assert!(
            read(&path)
                .unwrap()
                .disclose(|_| Ok(()))
                .unwrap()
                .is_empty()
        );
        assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
    }

    #[test]
    fn reading_unfinished_history_never_settles_or_repairs_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.jsonl");
        let before = bytes(&[prepared(&apply(1, CONTROL, absent(), dword(1)))]);
        std::fs::write(&path, &before).unwrap();
        let status =
            serde_json::to_value(read(&path).unwrap().disclose(|_| Ok(())).unwrap()).unwrap();
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
        assert!(
            read(&path)
                .unwrap()
                .disclose(|_| Ok(()))
                .unwrap()
                .is_empty()
        );
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
        let change = apply(1, CONTROL, dword(7), dword(1));
        assert!(decode(&bytes(&[finished(&change, Outcome::Applied)])).is_err());
        assert!(
            decode(&bytes(&[
                prepared(&change),
                finished(&change, Outcome::Unknown),
                prepared(&change)
            ]))
            .is_err()
        );
    }

    #[test]
    fn unknown_fields_duplicate_ids_and_cross_identity_history_are_refused() {
        let first = apply(1, CONTROL, dword(7), dword(1));
        let mut other = apply(2, CONTROL, dword(7), dword(1));
        other.owner = kernel::SecretRef::new("privacy", "another-fixture-owner").unwrap();
        assert!(
            decode(&bytes(&[
                prepared(&first),
                finished(&first, Outcome::Applied),
                prepared(&other)
            ]))
            .is_err()
        );
        assert!(
            decode(&bytes(&[
                prepared(&first),
                finished(&first, Outcome::Applied),
                prepared(&first)
            ]))
            .is_err()
        );
        let mut value = serde_json::to_value(prepared(&first)).unwrap();
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
        let prepared = prepared(&apply(1, CONTROL, dword(7), dword(1)));
        for (schema, owner) in [(SCHEMA, PRIVATE_INPUT), (2, "secret:privacy/fixture-owner")] {
            let mut line = serde_json::to_value(&prepared).unwrap();
            *line.pointer_mut("/schema").unwrap() = serde_json::json!(schema);
            *line.pointer_mut("/event/intent/owner").unwrap() = serde_json::json!(owner);
            let mut before = serde_json::to_vec(&line).unwrap();
            before.push(b'\n');
            std::fs::write(&path, &before).unwrap();
            assert!(read(&path).is_err(), "unsafe history was accepted");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn decoding_refusal_never_repeats_private_input() {
        let prepared = prepared(&apply(1, CONTROL, dword(7), dword(1)));
        for field in ["control", "unexpected", "owner"] {
            let mut line = serde_json::to_value(&prepared).unwrap();
            if field == "unexpected" {
                line.pointer_mut("/event/intent")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert(PRIVATE_INPUT.to_owned(), serde_json::json!(true));
            } else {
                *line.pointer_mut(&format!("/event/intent/{field}")).unwrap() =
                    serde_json::json!(PRIVATE_INPUT);
            }
            let mut before = serde_json::to_vec(&line).unwrap();
            before.push(b'\n');
            let fault = match decode(&before) {
                Ok(_) => panic!("private input accepted"),
                Err(fault) => fault,
            };
            assert!(!format!("{fault:?}").contains(PRIVATE_INPUT));
            let fault = fault.into_ax();
            assert!(!fault.to_string().contains(PRIVATE_INPUT));
            assert_eq!(fault.code(), &kernel::AxCode::StorageFatal);
            assert!(fault.subject().contains("column"));
        }
    }
}
