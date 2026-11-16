// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::vault::{KeyringVault, Vault};
use kernel::{AxCode, AxError, SecretRef};
use zeroize::Zeroizing;

const VERIFY: &str = "verify privacy owner";
const BIND: &str = "bind privacy owner";

pub fn verify_platform_identity(reference: &SecretRef, observed: &str) -> Result<(), AxError> {
    verify_identity_binding(reference, observed, |reference| KeyringVault.get(reference))
}

pub fn verify_identity_binding(
    reference: &SecretRef,
    observed: &str,
    read: impl FnOnce(&SecretRef) -> Result<Option<Zeroizing<String>>, AxError>,
) -> Result<(), AxError> {
    let stored = read(reference)
        .map_err(|source| Refusal::Unavailable(*source.code()).into_ax(VERIFY))?
        .ok_or_else(|| Refusal::Missing.into_ax(VERIFY))?;
    if observed.is_empty() || stored.as_str() != observed {
        return Err(Refusal::Foreign.into_ax(VERIFY));
    }
    Ok(())
}

pub fn bind_platform_identity(reference: &SecretRef, observed: &str) -> Result<(), AxError> {
    bind_identity(reference, observed, &mut KeyringVault)
}

pub(crate) fn bind_identity(
    reference: &SecretRef,
    observed: &str,
    vault: &mut impl Vault,
) -> Result<(), AxError> {
    if observed.is_empty() {
        return Err(Refusal::Foreign.into_ax(BIND));
    }
    let unavailable = |source: AxError| Refusal::Unavailable(*source.code()).into_ax(BIND);
    if vault.get(reference).map_err(unavailable)?.is_some() {
        return Err(Refusal::Occupied.into_ax(BIND));
    }
    vault
        .put(reference, Zeroizing::new(observed.to_owned()))
        .map_err(unavailable)
}

enum Refusal {

    Unavailable(AxCode),
    Missing,
    Foreign,

    Occupied,
}

impl Refusal {
    fn into_ax(self, action: &'static str) -> AxError {
        let (code, subject, recovery) = match self {
            Self::Unavailable(code) => (
                code,
                "platform identity binding unavailable",
                "unlock the platform credential service and ask again; the binding and history stay unchanged",
            ),
            Self::Missing => (
                AxCode::CredentialMissing,
                "privacy owner binding missing",
                "sign in to the account whose credential store holds the owner binding; the history stays unchanged",
            ),
            Self::Foreign => (
                AxCode::ConfigInvalid,
                "privacy history belongs to another identity",
                "run as the Windows account that owns this history; the binding and history stay unchanged",
            ),
            Self::Occupied => (
                AxCode::ConfigInvalid,
                "privacy owner reference already in use",
                "ask again; the next attempt draws a new reference and the existing binding stays unchanged",
            ),
        };
        AxError::failure(code, action, subject).with_recovery(recovery)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::super::vault::MemoryVault;
    use super::*;

    fn refusal(code: AxCode, subject: &str, recovery: &str) -> Result<(), AxError> {
        Err(AxError::failure(code, VERIFY, subject).with_recovery(recovery))
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
    fn a_binding_takes_a_fresh_reference_and_verifies_afterwards() {
        let reference = SecretRef::new("privacy", "owner-fixture").unwrap();
        let mut vault = MemoryVault::default();
        bind_identity(&reference, "fixture-owner", &mut vault).unwrap();
        assert_eq!(
            verify_identity_binding(&reference, "fixture-owner", |reference| vault
                .get(reference)),
            Ok(())
        );
        let refused = |subject: &str, recovery: &str| {
            Err(AxError::failure(AxCode::ConfigInvalid, BIND, subject).with_recovery(recovery))
        };
        assert_eq!(
            bind_identity(&reference, "fixture-foreign", &mut vault),
            refused(
                "privacy owner reference already in use",
                "ask again; the next attempt draws a new reference and the existing binding stays unchanged",
            )
        );
        assert_eq!(
            vault
                .get(&reference)
                .unwrap()
                .as_deref()
                .map(String::as_str),
            Some("fixture-owner")
        );
        let unused = SecretRef::new("privacy", "owner-unused").unwrap();
        assert_eq!(
            bind_identity(&unused, "", &mut vault),
            refused(
                "privacy history belongs to another identity",
                "run as the Windows account that owns this history; the binding and history stay unchanged",
            )
        );
        assert_eq!(vault.get(&unused).unwrap(), None);
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
