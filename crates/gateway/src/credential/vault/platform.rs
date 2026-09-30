// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The operating system's credential store on this target, and the name
//! a secret reference takes inside it (gateway-SPEC.md section 8-4).

use kernel::SecretRef;

/// The store-specific modifiers that fix where this reference lives.
pub(super) fn modifiers(_reference: &SecretRef) -> Vec<(&'static str, String)> {
    Vec::new()
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
