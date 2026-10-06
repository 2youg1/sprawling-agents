// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only privacy status (`crates/sprawling/spec/Privacy/Cli.lean`).

use accounting::home::Home;
use kernel::{AxCode, AxError, SecretRef};
use zeroize::Zeroizing;

/// Reads local operation receipts without creating or settling history,
/// and shows them only to the identity their owner reference is bound to.
///
/// # Errors
/// Identity sampling, owner binding, home detection, busy or malformed history,
/// IO, and JSON encoding failures.
pub fn status() -> Result<String, AxError> {
    status_at(
        &Home::detect()?.privacy_history(),
        super::identity::read,
        gateway::verify_platform_identity,
    )
}

/// The identity is sampled before the history is opened, so a failed
/// sample says nothing about the history, not even whether it is intact.
fn status_at(
    path: &std::path::Path,
    identity: impl FnOnce() -> Result<Zeroizing<String>, AxError>,
    verify: impl FnOnce(&SecretRef, &str) -> Result<(), AxError>,
) -> Result<String, AxError> {
    let observed = identity()?;
    let history =
        super::journal::read(path).map_err(|fault| fault.into_ax("read local privacy history"))?;
    let statuses = history.disclose(|owner| verify(owner, &observed))?;
    serde_json::to_string(statuses).map_err(|source| {
        AxError::failure(
            AxCode::InvalidArgs,
            "encode privacy status",
            source.to_string(),
        )
        .with_recovery("keep the history unchanged and report the encoding failure")
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;
    use std::path::Path;

    fn refused() -> AxError {
        AxError::failure(
            AxCode::ConfigInvalid,
            "verify privacy owner",
            "identity unavailable",
        )
        .with_recovery("leave history unchanged")
    }

    fn fixture(path: &Path) -> Vec<u8> {
        use super::super::state::fixtures::{apply, bytes, dword, prepared};
        use super::super::target::{RawValue, Snapshot};
        let bytes = bytes(&[prepared(&apply(
            1,
            wire::PrivacyControl::PowershellTelemetryOptout,
            Snapshot::Registry(RawValue::Absent),
            dword(1),
        ))]);
        std::fs::write(path, &bytes).unwrap();
        bytes
    }

    #[test]
    fn identity_failure_precedes_even_a_malformed_history_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        std::fs::write(&path, b"not JSON").unwrap();
        assert_eq!(
            status_at(
                &path,
                || Err(refused()),
                |_, _| panic!("vault must not be queried")
            ),
            Err(refused())
        );
    }

    #[test]
    fn authorized_summary_omits_values_and_empty_history_skips_vault() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        assert_eq!(
            status_at(
                &path,
                || Ok(Zeroizing::new("fixture".to_owned())),
                |_, _| panic!("empty history must not query vault")
            )
            .unwrap(),
            "[]"
        );
        let before = fixture(&path);
        let mut calls = 0;
        let summary = status_at(
            &path,
            || Ok(Zeroizing::new("fixture".to_owned())),
            |reference, observed| {
                assert_eq!(
                    reference,
                    &SecretRef::new("privacy", "fixture-owner").unwrap()
                );
                assert_eq!(observed, "fixture");
                calls += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(summary, r#"[{"operation":1,"outcome":"unresolved"}]"#);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    fn snapshot(steps: Vec<(u32, Vec<u8>, u8, bool)>) -> (Vec<u8>, usize) {
        use super::super::state::fixtures::{apply, finished, prepared, restore};
        use super::super::state::{Intent, Outcome};
        use super::super::target::{RawValue, Snapshot};
        let mut lines = Vec::new();
        let mut owned: Vec<Intent> = Vec::new();
        let mut count = 0usize;
        for (index, (kind, raw, outcome, restoring)) in steps.into_iter().enumerate() {
            let id = u64::try_from(index.checked_add(1).unwrap()).unwrap();
            let intent = match owned.last() {
                Some(previous) if restoring => restore(id, previous),
                Some(_) | None => apply(
                    id,
                    wire::PrivacyControl::PowershellTelemetryOptout,
                    owned
                        .last()
                        .map_or(Snapshot::Registry(RawValue::Absent), |previous| {
                            previous.modified.clone()
                        }),
                    Snapshot::Registry(RawValue::Present { kind, bytes: raw }),
                ),
            };
            if intent.original == intent.modified {
                break;
            }
            lines.push(prepared(&intent));
            count = count.checked_add(1).unwrap();
            if outcome == 3 {
                break;
            }
            let result = match outcome {
                0 if intent.restore_of.is_some() => Outcome::Restored,
                0 => Outcome::Applied,
                1 => Outcome::NotApplied,
                2 => Outcome::Unknown,
                _ => panic!("generator outcome outside its strategy"),
            };
            lines.push(finished(&intent, result));
            match result {
                Outcome::Applied => owned.push(intent),
                Outcome::Restored => {
                    owned.pop().unwrap();
                }
                Outcome::NotApplied | Outcome::RolledBack => (),
                Outcome::Unknown => break,
            }
        }
        (super::super::state::fixtures::bytes(&lines), count)
    }

    fn histories() -> impl proptest::strategy::Strategy<Value = (Vec<u8>, usize)> {
        use proptest::prelude::*;
        proptest::collection::vec(
            (
                any::<u32>(),
                proptest::collection::vec(any::<u8>(), 0..64),
                0u8..4,
                any::<bool>(),
            ),
            0..24,
        )
        .prop_map(snapshot)
    }

    proptest::proptest! {
        #[test]
        fn only_the_bound_identity_receives_a_summary(
            (bytes, count) in histories(), owner in proptest::num::u64::ANY,
            offset in 1u64..=u64::MAX,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("history.jsonl");
            std::fs::write(&path, &bytes).unwrap();
            let reference = SecretRef::new("privacy", "fixture-owner").unwrap();
            let stored = owner.to_string();
            let foreign = owner.wrapping_add(offset).to_string();
            for observed in [&stored, &foreign] {
                let mut reads = 0usize;
                let result = status_at(&path, || Ok(Zeroizing::new(observed.clone())),
                    |requested, observed| gateway::verify_identity_binding(requested, observed, |requested| {
                        reads = reads.checked_add(1).unwrap();
                        assert_eq!(requested, &reference);
                        Ok(Some(Zeroizing::new(stored.clone())))
                    }));
                if count == 0 || observed == &stored {
                    let values: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
                    proptest::prop_assert_eq!(values.as_array().unwrap().len(), count);
                } else {
                    proptest::prop_assert_eq!(*result.unwrap_err().code(), AxCode::ConfigInvalid);
                }
                proptest::prop_assert_eq!(reads, usize::from(count != 0));
                proptest::prop_assert_eq!(std::fs::read(&path).unwrap(), bytes.clone());
            }
        }
    }
}
