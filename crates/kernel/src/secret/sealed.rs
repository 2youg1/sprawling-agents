// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Sealed values: plaintext that cannot reach any sink.

use secrecy::{ExposeSecret, SecretBox};
use zeroize::{Zeroize, Zeroizing};

/// A value that cannot reach any sink: no Debug, no Display, no serde,
/// no Clone; drop zeroizes (secrecy::SecretBox). Expose call sites are
/// whitelisted by `xtask secret` (gateway::endpoint/native only, S3).
pub struct Sealed<T: Zeroize>(SecretBox<T>);

impl<T: Zeroize> Sealed<T> {
    pub fn new(value: Box<T>) -> Sealed<T> {
        Sealed(SecretBox::new(value))
    }

    /// The redemption borrow. Whitelist enforced by `xtask secret`; the
    /// type-level guarantee is that even a leaked borrow cannot be
    /// formatted or serialized through `Sealed` itself.
    pub fn expose(&self) -> &T {
        self.0.expose_secret()
    }
}

impl Sealed<String> {
    /// The one exit besides redemption: into the vault that will hold
    /// it. Consuming, so a caller cannot keep what it just handed over,
    /// and the result is still a type that zeroizes on drop.
    ///
    /// It lives here rather than at the enrolment site because that is
    /// what keeps the expose whitelist to three files: plaintext leaves
    /// this type in the module that defines the type.
    #[must_use]
    pub fn into_vault_value(self) -> Zeroizing<String> {
        Zeroizing::new(self.expose().clone())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    #[test]
    fn sealed_exposes_by_borrow_and_nothing_else() {
        let sealed = Sealed::new(Box::new("hunter2".to_owned()));
        assert_eq!(sealed.expose(), "hunter2");
        // No Debug/Display/Serialize/Clone impls exist — pinned by the
        // S2.11 trybuild case (formatting a Sealed does not compile).
    }
}
