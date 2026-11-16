// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

use super::{Authority, DeviceId, DeviceKey};
use crate::pairing::{decode, encode};

impl DeviceId {

    #[must_use]
    pub fn text(&self) -> String {
        encode(&self.0)
    }

    pub fn read(text: &str) -> Result<Self, AxError> {
        decode(text.trim())
            .filter(|bytes| encode(bytes) == text.trim())
            .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
            .map(Self)
            .ok_or_else(|| unreadable("a device id", text))
    }
}

impl DeviceKey {

    #[must_use]
    pub fn text(&self) -> String {
        encode(&self.0)
    }

    pub fn read(text: &str) -> Result<Self, AxError> {
        decode(text.trim())
            .map(Self)
            .ok_or_else(|| unreadable("a device key", "the key text"))
    }
}

impl Authority {

    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Act => "act",
        }
    }

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
