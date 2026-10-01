// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a device is written down: the text of its id and key, and the
//! word for its authority (crates/remote_access/Spec.lean §8-11).
//!
//! The device table and the ledger lines both read these spellings, so
//! they live beside the types they spell rather than in either reader.
//! The ids and keys take the base32 the pairing code and the city's
//! fingerprint already use, one alphabet for every byte string a person
//! may meet.

use kernel::{AxCode, AxError};

use super::{Authority, DeviceId, DeviceKey};
use crate::pairing::{decode, encode};

impl DeviceId {
    /// The id as 26 base32 symbols, lowercase.
    #[must_use]
    pub fn text(&self) -> String {
        encode(&self.0)
    }

    /// Reads [`Self::text`] back.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the text is not the spelling of 16 bytes.
    pub fn read(text: &str) -> Result<Self, AxError> {
        decode(text.trim())
            .filter(|bytes| encode(bytes) == text.trim())
            .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
            .map(Self)
            .ok_or_else(|| unreadable("a device id", text))
    }
}

impl DeviceKey {
    /// The key's bytes in base32.
    #[must_use]
    pub fn text(&self) -> String {
        encode(&self.0)
    }

    /// Reads [`Self::text`] back.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the text is not base32.
    pub fn read(text: &str) -> Result<Self, AxError> {
        decode(text.trim())
            .map(Self)
            .ok_or_else(|| unreadable("a device key", "the key text"))
    }
}

impl Authority {
    /// `watch` or `act`.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Act => "act",
        }
    }

    /// Reads [`Self::word`] back.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for any other word.
    pub fn from_word(word: &str) -> Result<Self, AxError> {
        [Self::Watch, Self::Act]
            .into_iter()
            .find(|each| each.word() == word.trim())
            .ok_or_else(|| unreadable("a device's authority", word))
    }
}

fn unreadable(what: &str, text: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        format!("read {what}"),
        text.chars().take(64).collect::<String>(),
    )
    .with_recovery(
        "the device table was changed by hand; revoke the device with `/remote revoke` and pair it again",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_spelling_reads_back_as_itself() {
        let id = DeviceId::from_entropy([9; 16]);
        let key = DeviceKey::from_bytes(vec![1, 2, 3, 250]);
        assert_eq!(
            (
                DeviceId::read(&id.text()).ok(),
                DeviceKey::read(&key.text()).ok(),
                Authority::from_word(Authority::Watch.word()).ok(),
                Authority::from_word(Authority::Act.word()).ok(),
            ),
            (
                Some(id),
                Some(key),
                Some(Authority::Watch),
                Some(Authority::Act)
            )
        );
    }

    #[test]
    fn a_short_id_and_an_unknown_word_are_refused() {
        assert_eq!(
            (
                DeviceId::read("abc").map_err(|err| *err.code()),
                Authority::from_word("admin").map_err(|err| *err.code()),
            ),
            (Err(AxCode::InvalidArgs), Err(AxCode::InvalidArgs))
        );
    }
}
