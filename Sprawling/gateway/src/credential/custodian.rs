// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::vault::{
    Described, EnvReader, FileVault, KeyringVault, MemoryVault, Persistence, Vault, env_key,
};

use kernel::event::record::{ProviderDegraded, VaultFellBack};
use kernel::{AxCode, AxError, Payload, Sealed, SecretRef};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    PlatformService,

    EncryptedFile,

    SessionMemory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Custody {
    pub store: Store,
    pub persistence: Persistence,

    pub refusal: Option<String>,
}

pub struct Custodian {
    backend: Box<dyn Vault + Send>,

    store: Store,

    refusal: Option<String>,
    env: EnvReader,
}

impl Store {
    fn source(self) -> &'static str {
        match self {
            Store::PlatformService => KeyringVault::SOURCE,
            Store::EncryptedFile => FileVault::SOURCE,
            Store::SessionMemory => MemoryVault::SOURCE,
        }
    }

    fn persistence(self) -> Persistence {
        match self {
            Store::PlatformService => KeyringVault::PERSISTENCE,
            Store::EncryptedFile => FileVault::PERSISTENCE,
            Store::SessionMemory => MemoryVault::PERSISTENCE,
        }
    }
}

impl Custodian {
    pub fn probe() -> (Custodian, Option<Payload>) {
        let probe_ref = match SecretRef::parse("secret:sprawling/startup-probe") {
            Ok(reference) => reference,
            Err(_) => return Custodian::fell_back("probe reference unparsable".to_owned()),
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
                None,
            ),
            Ok(_) => Custodian::fell_back("platform service returned a different value".to_owned()),
            Err(err) => Custodian::fell_back(err.subject().to_owned()),
        }
    }

    fn fell_back(reason: String) -> (Custodian, Option<Payload>) {
        let notice = degraded_payload(&reason);
        Custodian::in_session_memory(reason, notice)
    }

    fn in_session_memory(
        reason: String,
        notice: Result<Payload, AxError>,
    ) -> (Custodian, Option<Payload>) {
        let (refusal, notice) = match notice {
            Ok(payload) => (reason, Some(payload)),
            Err(err) => (
                format!(
                    "{reason}; the provider_degraded notice for it could not be encoded: {err}"
                ),
                None,
            ),
        };
        (
            Custodian::with_backend(
                Box::new(MemoryVault::default()),
                Store::SessionMemory,
                Some(refusal),
            ),
            notice,
        )
    }

    pub fn in_memory() -> Custodian {
        Custodian::with_backend(Box::new(MemoryVault::default()), Store::SessionMemory, None)
    }

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
            env: Box::new(|key| std::env::var(key)),
        }
    }

    pub fn with_env_reader(mut self, env: EnvReader) -> Custodian {
        self.env = env;
        self
    }

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
        if self.shaded(&key) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "store credential",
                format!("{reference} is shaded by the environment variable {key}"),
            )
            .with_recovery("unset the environment variable, then store again"));
        }
        self.backend.put(reference, value)
    }

    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError> {
        let key = env_key(reference);
        match (self.env)(&key) {
            Ok(value) if !value.is_empty() => return Ok(Sealed::new(Box::new(value))),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "resolve credential",
                    format!("{key} is set to a value that is not Unicode"),
                )
                .with_recovery(format!(
                    "set {key} to the key's text, or unset it so the stored credential answers"
                )));
            }

            Ok(_) | Err(std::env::VarError::NotPresent) => {}
        }
        match self.backend.get(reference)? {
            Some(value) if !value.is_empty() => {
                Ok(Sealed::new(Box::new(value.as_str().to_owned())))
            }

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

    pub fn forget(&mut self, reference: &SecretRef) -> Result<(), AxError> {
        let key = env_key(reference);
        if self.shaded(&key) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "forget credential",
                format!("{reference} is supplied by the environment variable {key}"),
            )
            .with_recovery(format!(
                "unset {key} in the environment the city starts from; \
                 the stored copy is kept until then"
            )));
        }
        self.backend.delete(reference)
    }

    fn shaded(&self, key: &str) -> bool {
        match (self.env)(key) {
            Ok(value) => !value.is_empty(),
            Err(std::env::VarError::NotUnicode(_)) => true,
            Err(std::env::VarError::NotPresent) => false,
        }
    }

    pub fn describe(&self, reference: &SecretRef) -> Described {
        let key = env_key(reference);
        let configured = match (self.env)(&key) {
            Ok(value) if !value.is_empty() => Some(true),

            Err(std::env::VarError::NotUnicode(_)) => Some(false),
            Ok(_) | Err(std::env::VarError::NotPresent) => None,
        };
        if let Some(configured) = configured {
            return Described {
                configured,
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

fn degraded_payload(reason: &str) -> Result<Payload, AxError> {
    Payload::of(&ProviderDegraded::VaultFellBack(VaultFellBack {
        component: "vault".to_owned(),
        fallback: "session-memory".to_owned(),
        persistence: Persistence::ThisProcess.as_str().to_owned(),
        reason: reason.to_owned(),
    }))
}

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
