// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only privacy status (`crates/sprawling/spec/Privacy/Cli.lean`).

use accounting::home::Home;
use kernel::{AxCode, AxError};

/// Reads local operation receipts without creating or settling history.
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

fn status_at(
    path: &std::path::Path,
    identity: impl FnOnce() -> Result<zeroize::Zeroizing<String>, AxError>,
    mut verify: impl FnMut(&kernel::SecretRef, &str) -> Result<(), AxError>,
) -> Result<String, AxError> {
    let observed = identity()?;
    let history = super::journal::read(path).map_err(super::state::HistoryFault::into_ax)?;
    if let Some(reference) = history.owner() {
        verify(reference, &observed)?;
    }
    serde_json::to_string(history.statuses()).map_err(|source| {
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
    use kernel::SecretRef;
    use std::path::Path;
    use zeroize::Zeroizing;

    fn refused() -> AxError {
        AxError::failure(
            AxCode::ConfigInvalid,
            "verify privacy owner",
            "identity unavailable",
        )
        .with_recovery("leave history unchanged")
    }

    fn fixture(path: &Path) -> Vec<u8> {
        use super::super::state::{Control, DEFINITION, Event, Intent, Line, RawValue, SCHEMA};
        let modified = RawValue::Present {
            kind: 1,
            bytes: vec![49, 0, 0, 0],
        };
        let mut bytes = serde_json::to_vec(&Line {
            schema: SCHEMA,
            event: Event::Prepared {
                intent: Intent {
                    operation: std::num::NonZeroU64::new(1).unwrap(),
                    control: Control::WindowsUserPowershellTelemetry,
                    definition: DEFINITION,
                    owner: SecretRef::new("privacy", "fixture-owner").unwrap(),
                    original: RawValue::Absent { key_existed: false },
                    modified: modified.clone(),
                    recommendation: modified,
                    restore_of: None,
                },
            },
        })
        .unwrap();
        bytes.push(b'\n');
        std::fs::write(path, &bytes).unwrap();
        bytes
    }

    #[test]
    fn foreign_owner_never_receives_history_summary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        let before = fixture(&path);
        assert!(
            status_at(
                &path,
                || Ok(Zeroizing::new("current-fixture".to_owned())),
                |_, _| Err(refused())
            )
            .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
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
}
