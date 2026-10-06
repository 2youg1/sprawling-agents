// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only owner verification (`crates/gateway/spec/Credential.lean` D27).

use super::vault::{KeyringVault, Vault};
use kernel::{AxCode, AxError, SecretRef};
use zeroize::Zeroizing;

/// Compares a sampled OS identity with a platform Vault binding without exposing it.
///
/// # Errors
/// Missing, mismatched or inaccessible bindings refuse disclosure. No probe or write occurs.
pub fn verify_platform_identity(reference: &SecretRef, observed: &str) -> Result<(), AxError> {
    verify_identity_binding(reference, observed, |reference| KeyringVault.get(reference))
}

/// Verifies an identity through a read-only binding source without returning its value.
///
/// # Errors
/// Missing, mismatched or inaccessible bindings refuse disclosure; source diagnostics are removed.
pub fn verify_identity_binding(
    reference: &SecretRef,
    observed: &str,
    read: impl FnOnce(&SecretRef) -> Result<Option<Zeroizing<String>>, AxError>,
) -> Result<(), AxError> {
    let stored = read(reference)
        .map_err(|source| refused(*source.code(), "platform identity binding unavailable"))?
        .ok_or_else(|| refused(AxCode::CredentialMissing, "privacy owner binding missing"))?;
    if observed.is_empty() || stored.as_str() != observed {
        return Err(refused(
            AxCode::ConfigInvalid,
            "privacy history belongs to another identity",
        ));
    }
    Ok(())
}

fn refused(code: AxCode, subject: &str) -> AxError {
    AxError::failure(code, "verify privacy owner", subject)
        .with_recovery("unlock the platform credential service and use the owning account; leave the binding and history unchanged")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::super::vault::MemoryVault;
    use super::*;
    use std::cell::Cell;

    #[test]
    fn binding_comparison_refuses_missing_foreign_and_empty_identities() {
        let reference = SecretRef::new("privacy", "fixture-owner").unwrap();
        let mut vault = MemoryVault::default();
        assert_eq!(
            verify_identity_binding(&reference, "fixture-owner", |reference| vault
                .get(reference))
            .unwrap_err()
            .code(),
            &AxCode::CredentialMissing
        );
        vault
            .put(&reference, Zeroizing::new("fixture-owner".to_owned()))
            .unwrap();
        assert!(
            verify_identity_binding(&reference, "fixture-owner", |reference| vault
                .get(reference))
            .is_ok()
        );
        for observed in ["fixture-foreign", ""] {
            assert_eq!(
                verify_identity_binding(&reference, observed, |reference| vault.get(reference))
                    .unwrap_err()
                    .code(),
                &AxCode::ConfigInvalid
            );
        }
        assert_eq!(
            vault.get(&reference).unwrap().unwrap().as_str(),
            "fixture-owner"
        );
    }

    struct RefusingVault {
        reads: Cell<u32>,
    }
    impl Vault for RefusingVault {
        fn get(&self, _: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError> {
            self.reads.set(self.reads.get().checked_add(1).unwrap());
            Err(
                AxError::failure(AxCode::ConfigInvalid, "fixture", "private-fixture-input")
                    .with_recovery("private-fixture-input"),
            )
        }
        fn put(&mut self, _: &SecretRef, _: Zeroizing<String>) -> Result<(), AxError> {
            panic!("must not write")
        }
        fn delete(&mut self, _: &SecretRef) -> Result<(), AxError> {
            panic!("must not delete")
        }
    }

    #[test]
    fn platform_refusal_preserves_code_but_never_private_diagnostics_or_writes() {
        let vault = RefusingVault {
            reads: Cell::new(0),
        };
        let reference = SecretRef::new("privacy", "fixture-owner").unwrap();
        let error = verify_identity_binding(&reference, "private-fixture-input", |reference| {
            vault.get(reference)
        })
        .unwrap_err();
        assert_eq!(error.code(), &AxCode::ConfigInvalid);
        assert!(!format!("{error:?}").contains("private-fixture-input"));
        assert_eq!(vault.reads.get(), 1);
    }
}
