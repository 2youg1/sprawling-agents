// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! The custodian: store, resolve, rotate.
//!
//! **Scanning foreign bytes for secret shapes and putting a marker
//! where one was is not here.** It is `runtime::redact`, which every
//! sink that writes model output already goes through, and a second
//! implementation here would be a second answer to "what must never be
//! printed". What this module owns is the other half: a value a person
//! deliberately handed the city, kept under the name the caller asked
//! for and never handed back except to the wire.

use super::oauth::degraded_payload;
use super::vault::{Described, EnvReader, KeyringVault, MemoryVault, Persistence, Vault, env_key};

use kernel::{AxCode, AxError, Payload, Sealed, SecretRef};
use zeroize::Zeroizing;

/// The inner seam: store, fetch, delete. Nothing else leaves the crate.
pub struct Custodian {
    backend: Box<dyn Vault + Send>,
    source: &'static str,
    persistence: Persistence,
    env: EnvReader,
}

impl Custodian {
    /// Startup probe: write-read-delete against the platform service;
    /// all candidates failing falls back to session memory and returns
    /// the `provider_degraded` payload for the ledger. Never silent.
    pub fn probe() -> (Custodian, Option<Payload>) {
        let probe_ref = match SecretRef::parse("secret:sprawling/startup-probe") {
            Ok(reference) => reference,
            Err(_) => {
                return (
                    Custodian::with_backend(
                        Box::new(MemoryVault::default()),
                        MemoryVault::SOURCE,
                        MemoryVault::PERSISTENCE,
                    ),
                    degraded_payload("probe reference unparsable"),
                );
            }
        };
        let mut candidate = KeyringVault;
        let round_trip = candidate
            .put(&probe_ref, Zeroizing::new("probe".to_owned()))
            .and_then(|()| candidate.get(&probe_ref))
            .and_then(|read| {
                candidate.delete(&probe_ref)?;
                Ok(read)
            });
        match round_trip {
            Ok(Some(read)) if read.as_str() == "probe" => (
                Custodian::with_backend(
                    Box::new(KeyringVault),
                    KeyringVault::SOURCE,
                    KeyringVault::PERSISTENCE,
                ),
                // No notice: the probe passed and this is the platform
                // service, on Linux included. A `provider_degraded` event
                // on every healthy Linux start would be a false alarm
                // repeated until nobody reads the kind. The grade this
                // target can honour is carried by `describe`, and the
                // moment it costs a person anything - a key the reboot
                // took - is answered by `resolve` below, in words.
                None,
            ),
            Ok(_) => (
                Custodian::with_backend(
                    Box::new(MemoryVault::default()),
                    MemoryVault::SOURCE,
                    MemoryVault::PERSISTENCE,
                ),
                degraded_payload("platform service returned a different value"),
            ),
            Err(err) => (
                Custodian::with_backend(
                    Box::new(MemoryVault::default()),
                    MemoryVault::SOURCE,
                    MemoryVault::PERSISTENCE,
                ),
                degraded_payload(err.subject()),
            ),
        }
    }

    /// Session-memory custodian (tests, headless fallback by choice).
    pub fn in_memory() -> Custodian {
        Custodian::with_backend(
            Box::new(MemoryVault::default()),
            MemoryVault::SOURCE,
            MemoryVault::PERSISTENCE,
        )
    }

    fn with_backend(
        backend: Box<dyn Vault + Send>,
        source: &'static str,
        persistence: Persistence,
    ) -> Custodian {
        Custodian {
            backend,
            source,
            persistence,
            env: Box::new(|key| std::env::var(key).ok()),
        }
    }

    /// Test seam: replace the read-only source reader.
    pub fn with_env_reader(mut self, env: EnvReader) -> Custodian {
        self.env = env;
        self
    }

    /// Stores a value. Empty is not a configuration; a shaded reference
    /// (read-only source active) refuses and names the shader. The input
    /// is `Zeroizing`, not `Sealed`: `Sealed` unseals only at the two
    /// wire redemption points, and custody is a store, not a sink —
    /// callers holding a `Sealed` keep it sealed all the way to the wire
    /// (the S4 command face converts inside its own boundary).
    pub fn set(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError> {
        if value.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "store credential",
                "empty value is not a configuration",
            )
            .with_recovery(
                "paste the credential into the settings field again; to remove one, \
                 delete it instead of storing an empty value",
            ));
        }
        let key = env_key(reference);
        if (self.env)(&key).is_some_and(|v| !v.is_empty()) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "store credential",
                format!("{reference} is shaded by the environment variable {key}"),
            )
            .with_recovery("unset the environment variable, then store again"));
        }
        self.backend.put(reference, value)
    }

    /// Redemption: resolve per operation. Rotation works because no
    /// second copy survives between operations.
    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError> {
        let key = env_key(reference);
        if let Some(value) = (self.env)(&key)
            && !value.is_empty()
        {
            return Ok(Sealed::new(Box::new(value)));
        }
        match self.backend.get(reference)? {
            Some(value) if !value.is_empty() => {
                Ok(Sealed::new(Box::new(value.as_str().to_owned())))
            }
            // The recovery carries the backend's grade, because the
            // commonest way to reach this arm on Linux is a reboot that
            // emptied the kernel keyring, and "store the credential"
            // alone reads as though the key was never entered.
            _ => Err(AxError::failure(
                AxCode::CredentialMissing,
                "resolve credential",
                reference.to_string(),
            )
            .with_recovery(format!(
                "store the credential, or set its environment variable: {}",
                self.persistence.consequence()
            ))),
        }
    }

    /// State you can render; the value stays unreachable.
    pub fn describe(&self, reference: &SecretRef) -> Described {
        let key = env_key(reference);
        if (self.env)(&key).is_some_and(|v| !v.is_empty()) {
            return Described {
                configured: true,
                source: format!("environment ({key})"),
                persistence: Persistence::ThisProcess,
                writable: false,
            };
        }
        let configured = matches!(self.backend.get(reference), Ok(Some(v)) if !v.is_empty());
        Described {
            configured,
            source: self.source.to_owned(),
            persistence: self.persistence,
            writable: true,
        }
    }

    pub fn persistence(&self) -> Persistence {
        self.persistence
    }
}

/// PKCE begin (RFC 7636, S256): pure construction — the browser visit
/// and the token POST are the caller's I/O. The verifier arrives from
/// the assembly's seeded randomness (kernel never samples).
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
    use super::*;
    fn sample_token() -> String {
        // Runtime-assembled: the repository never holds a complete
        // high-entropy literal at rest (xtask secret discipline).
        ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat()
    }

    /// The half of A13 this module owns: a value the person handed over
    /// goes in under the name the caller named, comes back sealed, and
    /// is never part of what `describe` renders. The other half - bytes
    /// that merely look like a key never reaching a sink - belongs to
    /// `runtime::redact`, and this crate holds no second copy of it.
    #[test]
    fn a13_a_stored_credential_is_redeemable_and_never_rendered() {
        let mut custodian = Custodian::in_memory();
        let reference = SecretRef::parse("secret:anthropic/api-key").unwrap();
        custodian
            .set(&reference, Zeroizing::new(sample_token()))
            .unwrap();
        assert!(custodian.resolve(&reference).is_ok());
        let described = custodian.describe(&reference);
        assert!(described.configured);
        assert_eq!(described.persistence, Persistence::ThisProcess);
        let rendered = format!("{} {}", described.source, described.configured);
        assert!(!rendered.contains("Zx9yQ2mK4pL7"), "{rendered}");
    }

    #[test]
    fn missing_and_empty_read_as_not_configured() {
        let mut custodian = Custodian::in_memory();
        let reference = SecretRef::parse("secret:acme/key").unwrap();
        let err = match custodian.resolve(&reference) {
            Err(err) => err,
            Ok(_) => panic!("missing credential must not resolve"),
        };
        assert_eq!(*err.code(), AxCode::CredentialMissing);
        // The recovery says what the backend can keep, so a key the
        // reboot took is not reported as a key nobody ever entered.
        // This custodian is session memory; a Linux one says the same
        // about the boot, out of the same enum.
        assert!(
            err.recovery()
                .ends_with(custodian.persistence().consequence()),
            "{}",
            err.recovery()
        );
        assert!(
            Persistence::ThisBoot.consequence().contains("reboots"),
            "the Linux grade has to name what a reboot does"
        );
        let err = custodian
            .set(&reference, Zeroizing::new(String::new()))
            .unwrap_err();
        assert_eq!(*err.code(), AxCode::InvalidArgs);
        assert!(!custodian.describe(&reference).configured);
    }

    #[test]
    fn the_environment_shades_and_set_refuses_naming_the_shader() {
        let mut custodian = Custodian::in_memory().with_env_reader(Box::new(|key| {
            (key == "SPRAWLING_SECRET_ACME_KEY").then(|| "from-env".to_owned())
        }));
        let reference = SecretRef::parse("secret:acme/key").unwrap();
        // Resolve serves the read-only source (sealed).
        assert!(custodian.resolve(&reference).is_ok());
        // Describe shows read-only.
        let described = custodian.describe(&reference);
        assert!(described.configured);
        assert!(!described.writable);
        assert!(described.source.contains("SPRAWLING_SECRET_ACME_KEY"));
        // Set refuses: it would look successful and change nothing.
        let err = custodian
            .set(&reference, Zeroizing::new("new".to_owned()))
            .unwrap_err();
        assert!(err.subject().contains("SPRAWLING_SECRET_ACME_KEY"));
    }

    #[test]
    fn rotation_is_next_operation_effective_because_nothing_caches() {
        let mut custodian = Custodian::in_memory();
        let reference = SecretRef::parse("secret:acme/rotating").unwrap();
        custodian
            .set(&reference, Zeroizing::new("one".to_owned()))
            .unwrap();
        assert!(custodian.resolve(&reference).is_ok());
        custodian
            .set(&reference, Zeroizing::new("two".to_owned()))
            .unwrap();
        // No copy survives between operations: the next resolve reads the
        // backend, so the rotated value is what the wire test would see.
        assert!(custodian.resolve(&reference).is_ok());
    }
}
