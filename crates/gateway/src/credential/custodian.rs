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

/// Which store a city writes secrets to.
///
/// A value rather than a sentence, because a dependency report and a
/// person reading it have to be able to match on the answer. The
/// encrypted file is a store this crate carries and no probe selects
/// yet: the passphrase that opens it is asked for at start-up, and that
/// wiring is its own change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    /// The platform's own credential service.
    PlatformService,
    /// An encrypted file on this machine, opened once per start.
    EncryptedFile,
    /// This process only.
    SessionMemory,
}

/// What the startup probe concluded: which store this city writes
/// secrets to, how long that store keeps them, and the platform
/// service's own refusal when it did not keep the value it was asked
/// to keep.
///
/// One value, because the three answers come from one round trip: a
/// report that read them separately could say the platform service
/// works and that everything restarts with nothing, which is the pair
/// of answers the caller cannot act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Custody {
    pub store: Store,
    pub persistence: Persistence,
    /// The service's refusal, in its own words. `None` where the
    /// service worked, and where this city chose the store itself.
    pub refusal: Option<String>,
}

/// The inner seam: store, fetch, delete. Nothing else leaves the crate.
pub struct Custodian {
    backend: Box<dyn Vault + Send>,
    /// Which store this city writes to. The name `describe` renders and
    /// the grade `resolve`'s recovery states are both derived from it,
    /// so the two read ports cannot answer with different stores.
    store: Store,
    /// The platform service's refusal, kept rather than derived: only
    /// the probe saw it, and `describe` renders state rather than a
    /// reason a person has to act on.
    refusal: Option<String>,
    env: EnvReader,
}

impl Store {
    /// The name `describe` renders, taken from the backend's own
    /// constant: a person debugging a lost key needs to know which
    /// service refused.
    fn source(self) -> &'static str {
        match self {
            Store::PlatformService => KeyringVault::SOURCE,
            Store::EncryptedFile => "encrypted-file",
            Store::SessionMemory => MemoryVault::SOURCE,
        }
    }

    /// The longest this store keeps a value on this target. The grade is
    /// a fact about the store rather than about the probe, and it is
    /// stated here once.
    fn persistence(self) -> Persistence {
        match self {
            Store::PlatformService => KeyringVault::PERSISTENCE,
            Store::EncryptedFile => Persistence::AcrossRebootsWithPassphrase,
            Store::SessionMemory => MemoryVault::PERSISTENCE,
        }
    }
}

impl Custodian {
    /// Startup probe: write-read-delete against the platform service;
    /// all candidates failing falls back to session memory and returns
    /// the `provider_degraded` payload for the ledger. Never silent.
    pub fn probe() -> (Custodian, Option<Payload>) {
        let probe_ref = match SecretRef::parse("secret:sprawling/startup-probe") {
            Ok(reference) => reference,
            Err(_) => {
                let reason = "probe reference unparsable";
                return (
                    Custodian::with_backend(
                        Box::new(MemoryVault::default()),
                        Store::SessionMemory,
                        Some(reason.to_owned()),
                    ),
                    degraded_payload(reason),
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
                Custodian::with_backend(Box::new(KeyringVault), Store::PlatformService, None),
                // No notice: the probe passed and this is the platform
                // service, on Linux included. A `provider_degraded` event
                // on every healthy Linux start would be a false alarm
                // repeated until nobody reads the kind. The grade this
                // target can honour is carried by `describe`, and the
                // moment it costs a person anything - a key the reboot
                // took - is answered by `resolve` below, in words.
                None,
            ),
            Ok(_) => {
                let reason = "platform service returned a different value";
                (
                    Custodian::with_backend(
                        Box::new(MemoryVault::default()),
                        Store::SessionMemory,
                        Some(reason.to_owned()),
                    ),
                    degraded_payload(reason),
                )
            }
            Err(err) => (
                Custodian::with_backend(
                    Box::new(MemoryVault::default()),
                    Store::SessionMemory,
                    Some(err.subject().to_owned()),
                ),
                degraded_payload(err.subject()),
            ),
        }
    }

    /// Session-memory custodian (tests, headless fallback by choice).
    pub fn in_memory() -> Custodian {
        Custodian::with_backend(Box::new(MemoryVault::default()), Store::SessionMemory, None)
    }

    /// What the startup probe concluded, for a report about this
    /// machine. Built from the fields the two read ports read, so a
    /// caller cannot be told a store that neither `describe` nor
    /// `resolve` is using.
    pub fn custody(&self) -> Custody {
        Custody {
            store: self.store,
            persistence: self.store.persistence(),
            refusal: self.refusal.clone(),
        }
    }

    fn with_backend(
        backend: Box<dyn Vault + Send>,
        store: Store,
        refusal: Option<String>,
    ) -> Custodian {
        Custodian {
            backend,
            store,
            refusal,
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
                self.store.persistence().consequence()
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
            source: self.store.source().to_owned(),
            persistence: self.store.persistence(),
            writable: true,
        }
    }

    pub fn persistence(&self) -> Persistence {
        self.store.persistence()
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
mod tests;
