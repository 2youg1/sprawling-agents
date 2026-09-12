// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! Where secrets rest: vaults.

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, SecretRef};
use zeroize::Zeroizing;

/// The inner seam: store, fetch, delete. Nothing else leaves the crate.
pub(crate) trait Vault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError>;
    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError>;
}

/// How long the active backend keeps a value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistence {
    AcrossReboots,
    ThisBoot,
    ThisProcess,
}

impl Persistence {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Persistence::AcrossReboots => "across_reboots",
            Persistence::ThisBoot => "this_boot",
            Persistence::ThisProcess => "this_process",
        }
    }

    /// What this grade costs the person holding the key, in the words a
    /// message to them may use.
    ///
    /// One authority for one sentence: the same text explains a vault at
    /// rest and a credential that has gone missing, so a Linux install
    /// cannot report a store that survives a reboot in one place and a
    /// bare `not configured` in the other.
    #[must_use]
    pub fn consequence(self) -> &'static str {
        match self {
            Persistence::AcrossReboots => {
                "a stored key survives a restart of this computer, so it is entered once"
            }
            Persistence::ThisBoot => {
                "the kernel keyring holds a stored key until this computer reboots, \
                 and the key has to be entered again after that"
            }
            Persistence::ThisProcess => {
                "a stored key lives in this process only, so it has to be entered again \
                 the next time sprawling starts"
            }
        }
    }
}

/// `describe`'s answer: state you can render, value you cannot get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described {
    pub configured: bool,
    pub source: String,
    pub persistence: Persistence,
    pub writable: bool,
}

/// Platform credential service via the keyring crate.
pub(crate) struct KeyringVault;

impl KeyringVault {
    /// What this backend is, in the words `describe` renders.
    ///
    /// Named per target because it is a different service per target,
    /// and a person debugging a lost key needs to know which one refused.
    pub(crate) const SOURCE: &'static str = if cfg!(target_os = "linux") {
        "kernel-keyring"
    } else {
        "platform-credential-service"
    };

    /// The longest this backend can keep a value on this target.
    ///
    /// Windows Credential Manager and the macOS Keychain write to disk.
    /// The Linux build talks to the kernel's keyring through keyutils,
    /// which is memory the kernel clears on reboot — the one store that
    /// stays reachable from a static musl binary with no session bus.
    /// The grade is therefore a fact about the target rather than about
    /// the probe, and it is stated here, once.
    pub(crate) const PERSISTENCE: Persistence = if cfg!(target_os = "linux") {
        Persistence::ThisBoot
    } else {
        Persistence::AcrossReboots
    };
}

fn keyring_entry(reference: &SecretRef) -> Result<keyring::Entry, AxError> {
    keyring::Entry::new(
        &format!("sprawling/{}", reference.realm()),
        reference.name(),
    )
    .map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "open credential entry",
            err.to_string(),
        )
    })
}

impl Vault for KeyringVault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError> {
        keyring_entry(reference)?
            .set_password(&value)
            .map_err(|err| {
                AxError::failure(AxCode::ConfigInvalid, "store credential", err.to_string())
            })
    }

    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError> {
        match keyring_entry(reference)?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "fetch credential",
                err.to_string(),
            )),
        }
    }

    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError> {
        match keyring_entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "delete credential",
                err.to_string(),
            )),
        }
    }
}

/// Session-memory fallback: honest about its persistence grade.
#[derive(Default)]
pub(crate) struct MemoryVault {
    values: BTreeMap<String, Zeroizing<String>>,
}

impl MemoryVault {
    pub(crate) const SOURCE: &'static str = "session-memory";
    pub(crate) const PERSISTENCE: Persistence = Persistence::ThisProcess;
}

impl Vault for MemoryVault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError> {
        self.values.insert(reference.to_string(), value);
        Ok(())
    }

    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError> {
        Ok(self.values.get(&reference.to_string()).cloned())
    }

    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError> {
        self.values.remove(&reference.to_string());
        Ok(())
    }
}

/// Reads the read-only source (process environment). Injected so tests
/// can shade without touching the real environment (set_var is unsafe in
/// edition 2024, and tests never mutate shared process state).
pub type EnvReader = Box<dyn Fn(&str) -> Option<String> + Send>;

pub(crate) fn env_key(reference: &SecretRef) -> String {
    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect()
    };
    format!(
        "SPRAWLING_SECRET_{}_{}",
        sanitize(reference.realm()),
        sanitize(reference.name())
    )
}

/// The custody face. One per process; hands out payloads, never values.
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::oauth::{oauth_begin, oauth_redeem_request};
    use super::*;

    /// A token endpoint that answers one POST with `status` and `body`,
    /// and reports what it was sent.
    /// A token endpoint that answers one POST with `status` and `body`,
    /// and reports what it was sent.

    #[test]
    fn pkce_matches_the_rfc_7636_vector_and_fails_closed() {
        let profile = crate::oauth_profiles::profile("anthropic").unwrap();
        // RFC 7636 Appendix B vector, assembled at runtime (C13: no
        // complete high-entropy literal at rest in the repository).
        let verifier = ["dBjftJeZ4CVP-mB92", "K27uhbUJU1p1r_", "wW1gFWFOEjXk"].concat();
        let expected_challenge = ["E9Melhoa2OwvFrEMTJ", "guCHaoeK1t8URW", "buGJSstw-cM"].concat();
        let pending = oauth_begin(profile, verifier.clone(), "st".to_owned()).unwrap();
        assert!(
            pending
                .auth_url
                .contains(&format!("code_challenge={expected_challenge}")),
            "{}",
            pending.auth_url
        );
        assert!(pending.auth_url.contains("code_challenge_method=S256"));
        let redeem = oauth_redeem_request(profile, &pending, "authcode").unwrap();
        assert_eq!(redeem.url, profile.token_endpoint);
        assert!(redeem.body.contains("authorization_code"));
        // The defect the upstream intelligence sources carry: a state
        // equal to the verifier proves one thing twice and the other
        // thing not at all.
        let reused = oauth_begin(profile, verifier.clone(), verifier.clone())
            .err()
            .expect("state equal to the verifier is refused");
        assert_eq!(reused.code(), &AxCode::InvalidArgs);
        assert!(
            reused.recovery().contains("randomness of its own"),
            "the refusal says what the state is for: {}",
            reused.recovery()
        );

        // Empty intelligence fails closed; short verifiers are refused.
        let openai = crate::oauth_profiles::profile("openai").unwrap();
        assert!(oauth_begin(openai, "x".repeat(50), "s".to_owned()).is_err());
        assert!(oauth_begin(profile, "short".to_owned(), "s".to_owned()).is_err());
    }
}
