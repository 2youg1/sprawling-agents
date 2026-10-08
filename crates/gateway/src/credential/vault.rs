// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! Where secrets rest: vaults.

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, SecretRef};
use zeroize::Zeroizing;

// The third backend. `dead_code` rather than `expect`: the module is
// complete and its one production caller is the backend choice in
// `Custodian::probe`, which lands with the passphrase prompt at start-up;
// the test build does construct it, so an expectation here would be
// unfulfilled in one of the two configurations and warn about that
// instead.
#[allow(
    dead_code,
    reason = "the passphrase backend awaits its one caller in Custodian::probe"
)]
mod file;
mod platform;

pub(crate) use file::FileVault;

/// The inner seam: store, fetch, delete. Nothing else leaves the crate.
pub(crate) trait Vault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError>;
    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError>;
}

/// How long the active backend keeps a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistence {
    AcrossReboots,
    /// An encrypted file on this machine, opened once per start with a
    /// passphrase. Below `AcrossReboots` because it costs the person
    /// something every start, and above `ThisBoot` because what it
    /// costs is a passphrase rather than every credential again.
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

/// `describe`'s answer: state you can render, value you cannot get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described {
    pub configured: bool,
    pub source: String,
    pub persistence: Persistence,
    pub writable: bool,
}

/// The platform credential service, reached through keyring-core and
/// this target's store (`platform`).
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
///
/// The answer is `std::env::var`'s own, so a variable that is set to a
/// value that is not Unicode stays apart from one that is not set
/// (gateway D24).
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
