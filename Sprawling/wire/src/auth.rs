// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, B3Hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pairing {
    Held,
    Absent,
}

const ALPHABET: &[u8; 29] = b"2346789abcdefghjkmnpqrtuvwxyz";

const GROUP_LEN: usize = 5;
const GROUP_COUNT: usize = 4;

const MIN_CONFIGURED_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingToken(B3Hash);

impl PairingToken {

    #[must_use]
    pub fn mint(entropy: [u8; 32]) -> (Self, String) {
        let text = spell(&entropy, GROUP_LEN, GROUP_COUNT);
        let token = Self(B3Hash::digest(text.as_bytes()));
        (token, text)
    }

    pub fn from_configured(raw: &str) -> Result<Self, AxError> {
        if raw.chars().count() < MIN_CONFIGURED_LEN {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "adopt a configured pairing token",
                format!("a pairing token needs at least {MIN_CONFIGURED_LEN} characters"),
            )
            .with_recovery("let the host mint one, or configure a longer value"));
        }
        Ok(Self(B3Hash::digest(raw.as_bytes())))
    }

    #[must_use]
    pub fn digest(&self) -> B3Hash {
        self.0
    }
}

pub(crate) fn spell(entropy: &[u8], group_len: usize, groups: usize) -> String {
    let mut text = String::new();
    for (index, byte) in entropy
        .iter()
        .take(group_len.saturating_mul(groups))
        .enumerate()
    {
        if index != 0 && index.checked_rem(group_len) == Some(0) {
            text.push('-');
        }
        let slot = usize::from(*byte)
            .checked_rem(ALPHABET.len())
            .unwrap_or_default();
        if let Some(symbol) = ALPHABET.get(slot) {
            text.push(char::from(*symbol));
        }
    }
    text
}

#[must_use]
pub fn verify(presented: Option<&str>, expected: &B3Hash) -> bool {
    let Some(presented) = presented else {
        return false;
    };
    let offered = B3Hash::digest(presented.as_bytes());
    let mut difference = 0u8;
    for (a, b) in offered.as_bytes().iter().zip(expected.as_bytes().iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn the_alphabet_omits_every_pair_a_person_would_confuse() {
        for confusable in *b"0O1lI5S" {
            assert!(
                !ALPHABET.contains(&confusable),
                "{} is transcribed wrong often enough to matter",
                char::from(confusable)
            );
        }
        let mut deduped = ALPHABET.to_vec();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(deduped.len(), ALPHABET.len(), "no symbol appears twice");
    }

    #[test]
    fn a_minted_code_has_the_shape_the_constants_promise() {
        let (_token, shown) = PairingToken::mint([0x44u8; 32]);
        let groups: Vec<&str> = shown.split('-').collect();
        assert_eq!(groups.len(), GROUP_COUNT);
        for group in groups {
            assert_eq!(group.chars().count(), GROUP_LEN);
        }
    }

    #[test]
    fn verification_is_blind_to_how_long_the_wrong_guess_was() {
        let (token, _shown) = PairingToken::mint([0x55u8; 32]);
        let expected = token.digest();
        assert!(!verify(Some(""), &expected));
        assert!(!verify(Some("a"), &expected));
        assert!(!verify(Some(&"a".repeat(4096)), &expected));
    }
}
