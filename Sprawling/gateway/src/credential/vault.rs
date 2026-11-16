// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, SecretRef};
use zeroize::Zeroizing;

#[allow(
    dead_code,
    reason = "the passphrase backend awaits its one caller in Custodian::probe"
)]
mod file;
mod platform;

pub(crate) use file::FileVault;

pub(crate) trait Vault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError>;
    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistence {
    AcrossReboots,

    AcrossRebootsWithPassphrase,
    ThisBoot,
    ThisProcess,
}

impl Persistence {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Persistence::AcrossReboots => "across_reboots",
            Persistence::AcrossRebootsWithPassphrase => "across_reboots_with_passphrase",
            Persistence::ThisBoot => "this_boot",
            Persistence::ThisProcess => "this_process",
        }
    }

    #[must_use]
    pub fn consequence(self) -> &'static str {
        match self {
            Persistence::AcrossReboots => {
                "a stored key survives a restart of this computer, so it is entered once"
            }
            Persistence::AcrossRebootsWithPassphrase => {
                "a stored key lives in an encrypted file on this computer, so the \
                 passphrase that opens it is entered once each time sprawling starts"
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described {
    pub configured: bool,
    pub source: String,
    pub persistence: Persistence,
    pub writable: bool,
}

pub(crate) struct KeyringVault;

impl KeyringVault {

    pub(crate) const SOURCE: &'static str = if cfg!(target_os = "linux") {
        "kernel-keyring"
    } else {
        "platform-credential-service"
    };

    pub(crate) const PERSISTENCE: Persistence = if cfg!(target_os = "linux") {
        Persistence::ThisBoot
    } else {
        Persistence::AcrossReboots
    };
}

impl Vault for KeyringVault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError> {
        platform::entry(reference)?
            .set_password(&value)
            .map_err(|err| {
                AxError::failure(AxCode::ConfigInvalid, "store credential", err.to_string())
                    .with_recovery(
                        "unlock this machine's credential service and store the value again; \
                         sprawling writes plaintext nowhere else",
                    )
            })
    }

    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError> {
        match platform::entry(reference)?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(err) => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "fetch credential",
                err.to_string(),
            )
            .with_recovery(
                "unlock this machine's credential service, or store the credential again \
                 from the settings page",
            )),
        }
    }

    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError> {
        match platform::entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(err) => {
                Err(
                    AxError::failure(AxCode::ConfigInvalid, "delete credential", err.to_string())
                        .with_recovery(
                            "unlock this machine's credential service and delete the entry again, \
                 or remove it in that service's own window",
                        ),
                )
            }
        }
    }
}

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

pub type EnvReader = Box<dyn Fn(&str) -> Result<String, std::env::VarError> + Send>;

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
        "{}{}_{}",
        child::SECRET_PREFIX,
        sanitize(reference.realm()),
        sanitize(reference.name())
    )
}
