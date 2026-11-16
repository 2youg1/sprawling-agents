// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use secrecy::{ExposeSecret, SecretBox};
use zeroize::{Zeroize, Zeroizing};

pub struct Sealed<T: Zeroize>(SecretBox<T>);

impl<T: Zeroize> Sealed<T> {
    pub fn new(value: Box<T>) -> Sealed<T> {
        Sealed(SecretBox::new(value))
    }

    pub fn expose(&self) -> &T {
        self.0.expose_secret()
    }
}

impl Sealed<String> {

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
    fn the_vault_receives_the_sealed_secret() {
        let sealed = Sealed::new(Box::new("hunter2".to_owned()));
        assert_eq!(sealed.into_vault_value().as_str(), "hunter2");
    }

    #[test]
    fn sealed_exposes_by_borrow_and_nothing_else() {
        let sealed = Sealed::new(Box::new("hunter2".to_owned()));
        assert_eq!(sealed.expose(), "hunter2");

    }
}
