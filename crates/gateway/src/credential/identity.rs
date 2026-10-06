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
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::super::vault::MemoryVault;
    use super::*;

    fn refusal(code: AxCode, subject: &str, recovery: &str) -> Result<(), AxError> {
        Err(AxError::failure(code, "verify privacy owner", subject).with_recovery(recovery))
    }

    #[test]
    fn binding_comparison_refuses_missing_foreign_and_empty_identities() {
        let reference = SecretRef::new("privacy", "fixture-owner").unwrap();
        let mut vault = MemoryVault::default();
        let verify = |vault: &MemoryVault, observed: &str| {
            verify_identity_binding(&reference, observed, |reference| vault.get(reference))
        };
        assert_eq!(
            verify(&vault, "fixture-owner"),
            refusal(
                AxCode::CredentialMissing,
                "privacy owner binding missing",
                "sign in to the account whose credential store holds the owner binding; the history stays unchanged",
            )
        );
        vault
            .put(&reference, Zeroizing::new("fixture-owner".to_owned()))
            .unwrap();
        assert_eq!(verify(&vault, "fixture-owner"), Ok(()));
        for observed in ["fixture-foreign", "fixture-owner ", ""] {
            assert_eq!(
                verify(&vault, observed),
                refusal(
                    AxCode::ConfigInvalid,
                    "privacy history belongs to another identity",
                    "run as the Windows account that owns this history; the binding and history stay unchanged",
                )
            );
        }
    }

    #[test]
    fn a_source_refusal_keeps_its_code_and_drops_its_words() {
        let reference = SecretRef::new("privacy", "fixture-owner").unwrap();
        let refused = verify_identity_binding(&reference, "private-fixture-input", |_| {
            Err(
                AxError::failure(AxCode::StorageFatal, "fixture", "private-fixture-input")
                    .with_recovery("private-fixture-input"),
            )
        });
        assert_eq!(
            refused,
            refusal(
                AxCode::StorageFatal,
                "platform identity binding unavailable",
                "unlock the platform credential service and ask again; the binding and history stay unchanged",
            )
        );
    }
}
