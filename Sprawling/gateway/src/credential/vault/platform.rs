// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::HashMap;

use kernel::{AxCode, AxError, SecretRef};
use keyring_core::api::CredentialStoreApi as _;

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
            "start this machine's credential service (Keychain on macOS, Credential \
             Manager on Windows, the kernel keyring on Linux), then try again",
        )
    })
}

fn service(reference: &SecretRef) -> String {
    format!("sprawling/{}", reference.realm())
}

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

#[cfg(all(test, any(windows, target_os = "linux")))]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn on_windows_a_reference_keeps_the_target_name_keyring_3_wrote() {
        let reference = SecretRef::parse("secret:openai/key").unwrap();
        assert_eq!(
            modifiers(&reference),
            vec![("target", "key.sprawling/openai".to_owned())]
        );
    }

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
