// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::B3Hash;

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

const SYMBOL_BITS: u32 = 5;

const GROUP_LEN: usize = 5;

pub const CODE_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingCode(B3Hash);

impl PairingCode {

    #[must_use]
    pub fn mint(entropy: [u8; CODE_BYTES]) -> (Self, String) {
        let canonical = encode(&entropy);
        let shown = canonical
            .as_bytes()
            .chunks(GROUP_LEN)
            .map(|group| {
                group
                    .iter()
                    .map(|byte| char::from(*byte))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("-");
        (Self(B3Hash::digest(canonical.as_bytes())), shown)
    }

    #[must_use]
    pub fn read(typed: &str) -> Self {
        Self(B3Hash::digest(canonical(typed).as_bytes()))
    }
}

pub(crate) fn canonical(typed: &str) -> String {
    typed
        .chars()
        .filter(|each| !each.is_whitespace() && *each != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn is_symbol(byte: u8) -> bool {
    ALPHABET.contains(&byte)
}

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut held: u32 = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        held = held.saturating_add(8);
        while held >= SYMBOL_BITS {
            held = held.saturating_sub(SYMBOL_BITS);
            push_symbol(&mut out, buffer >> held);
        }
        buffer &= (1u32 << held).wrapping_sub(1);
    }
    if held > 0 {
        push_symbol(&mut out, buffer << SYMBOL_BITS.saturating_sub(held));
    }
    out
}

pub(crate) fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut held: u32 = 0;
    for symbol in text.bytes() {
        let value = ALPHABET
            .iter()
            .zip(0u32..)
            .find_map(|(each, value)| (*each == symbol).then_some(value))?;
        buffer = (buffer << SYMBOL_BITS) | value;
        held = held.saturating_add(SYMBOL_BITS);
        if held >= 8 {
            held = held.saturating_sub(8);
            let [.., byte] = (buffer >> held).to_be_bytes();
            out.push(byte);
        }
        buffer &= (1u32 << held).wrapping_sub(1);
    }
    Some(out)
}

fn push_symbol(out: &mut String, value: u32) {
    let slot = usize::try_from(value & 0b1_1111).unwrap_or_default();
    if let Some(symbol) = ALPHABET.get(slot) {
        out.push(char::from(*symbol));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_minted_code_reads_back_however_it_was_retyped() {
        let (code, shown) = PairingCode::mint([7; CODE_BYTES]);
        let squashed: String = shown.chars().filter(|each| *each != '-').collect();
        assert_eq!(
            [
                PairingCode::read(&shown),
                PairingCode::read(&shown.to_uppercase()),
                PairingCode::read(&format!("  {squashed} ")),
            ],
            [code, code, code]
        );
    }

    #[test]
    fn the_code_is_rfc_4648_base32_of_its_entropy() {

        assert_eq!(encode(b"foobar"), "mzxw6ytboi");
        let (_, shown) = PairingCode::mint([0xff; CODE_BYTES]);
        assert_eq!(shown.chars().filter(|each| *each != '-').count(), 26);
    }

    #[test]
    fn base32_decodes_what_it_encodes() {
        assert_eq!(
            [decode("mzxw6ytboi"), decode("mzxw6ytbo1")],
            [Some(b"foobar".to_vec()), None]
        );
    }

    #[test]
    fn two_entropies_give_two_codes() {
        assert_ne!(
            PairingCode::mint([1; CODE_BYTES]).0,
            PairingCode::mint([2; CODE_BYTES]).0
        );
    }
}
