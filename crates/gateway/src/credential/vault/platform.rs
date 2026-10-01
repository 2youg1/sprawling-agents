// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The operating system's credential store on this target, and the name
//! a secret reference takes inside it (`crates/gateway/spec/Credential.lean` §8-4).

use std::collections::HashMap;

use kernel::{AxCode, AxError, SecretRef};
use keyring_core::api::CredentialStoreApi as _;

/// The entry `reference` names in this target's credential store.
///
/// The store is built on every call rather than set as the process's
/// default store, so no code outside the vault can change where a key
/// lands. Building an entry touches nothing in the store; setting,
/// reading or deleting it does.
///
/// # Errors
/// `E_CONFIG_INVALID` when the store cannot be opened or refuses the
/// name, with the recovery that names this target's service.
pub(super) fn entry(reference: &SecretRef) -> Result<keyring_core::Entry, AxError> {
    let service = service(reference);
    let named = modifiers(reference);
    let modifiers: HashMap<&str, &str> = named
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    built(&service, reference.name(), &modifiers).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "open credential entry",
            err.to_string(),
        )
        .with_recovery(
            "start this machine's credential service (Keychain on macOS, Credential              Manager on Windows, the kernel keyring on Linux), then try again",
        )
    })
}

/// The service a reference's realm is kept under, on every target.
fn service(reference: &SecretRef) -> String {
    format!("sprawling/{}", reference.realm())
}

/// The store-specific modifiers that fix where this reference lives.
///
/// They spell the names keyring 3 wrote, so a key stored before the
/// move to keyring-core is found where it was left: the Windows target
/// name is `<name>.<service>`, and the Linux keyutils description is
/// `keyring-rs:<name>@<service>`. The keychain on macOS is addressed by
/// service and account alone, so it takes no modifier.
pub(super) fn modifiers(reference: &SecretRef) -> Vec<(&'static str, String)> {
    let (name, service) = (reference.name(), service(reference));
    if cfg!(windows) {
        vec![("target", format!("{name}.{service}"))]
    } else if cfg!(target_os = "linux") {
        vec![("description", format!("keyring-rs:{name}@{service}"))]
    } else {
        Vec::new()
    }
}

#[cfg(windows)]
fn built(
    service: &str,
    user: &str,
    modifiers: &HashMap<&str, &str>,
) -> keyring_core::Result<keyring_core::Entry> {
    windows_native_keyring_store::Store::new()?.build(service, user, Some(modifiers))
}

#[cfg(target_os = "macos")]
fn built(
    service: &str,
    user: &str,
    modifiers: &HashMap<&str, &str>,
) -> keyring_core::Result<keyring_core::Entry> {
    apple_native_keyring_store::keychain::Store::new()?.build(service, user, Some(modifiers))
}

#[cfg(target_os = "linux")]
fn built(
    service: &str,
    user: &str,
    modifiers: &HashMap<&str, &str>,
) -> keyring_core::Result<keyring_core::Entry> {
    linux_keyutils_keyring_store::Store::new()?.build(service, user, Some(modifiers))
}

/// A target with no credential store this build links: the probe reads
/// the refusal and keeps the city's secrets in session memory.
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn built(
    _service: &str,
    _user: &str,
    _modifiers: &HashMap<&str, &str>,
) -> keyring_core::Result<keyring_core::Entry> {
    Err(keyring_core::Error::NotSupportedByStore(
        "this target has no credential store in this build".to_owned(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    /// Windows Credential Manager finds a credential by its target name
    /// alone, so a key a person stored before the move to keyring-core
    /// is found only under the name keyring 3 gave it.
    #[cfg(windows)]
    #[test]
    fn on_windows_a_reference_keeps_the_target_name_keyring_3_wrote() {
        let reference = SecretRef::parse("secret:openai/key").unwrap();
        assert_eq!(
            modifiers(&reference),
            vec![("target", "key.sprawling/openai".to_owned())]
        );
    }

    /// The kernel keyring finds a key by its description, and the
    /// keyutils store's own default prefix is not the one keyring 3 wrote.
    #[cfg(target_os = "linux")]
    #[test]
    fn on_linux_a_reference_keeps_the_description_keyring_3_wrote() {
        let reference = SecretRef::parse("secret:openai/key").unwrap();
        assert_eq!(
            modifiers(&reference),
            vec![("description", "keyring-rs:key@sprawling/openai".to_owned())]
        );
    }
}
