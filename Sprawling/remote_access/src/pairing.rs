// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one-time code that pairs a device, from minting to the form a
//! person copies (crates/remote_access/Spec.lean §8-2).
//!
//! The door keeps only the code's digest. The text goes back to the
//! caller, who shows it once, in a QR code and as characters; nothing in
//! this crate holds it afterwards. Entropy arrives as a parameter, so the
//! assembly layer is the one place a random byte is drawn.

use kernel::B3Hash;

/// RFC 4648 base32 in lower case: a QR code reader and a clipboard both
/// carry it unchanged, and it has no symbol a URL must escape.
const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Bits one symbol carries.
const SYMBOL_BITS: u32 = 5;

/// Symbols per group in the form a person reads.
const GROUP_LEN: usize = 5;

/// The bytes of entropy one code is minted from: 128 bits, which no
/// guessing within the ten minutes a code lives can approach.
pub const CODE_BYTES: usize = 16;

/// A pairing code, held as the digest of its canonical text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingCode(B3Hash);

impl PairingCode {
    /// Mints a code from caller-supplied entropy and returns it with the
    /// text to show once, grouped for reading (`abcde-fghij-…`).
    ///
    /// Deterministic in `entropy`: the same bytes give the same code.
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

    /// The code a person typed or pasted, read the way it was shown:
    /// case, spaces and dashes do not matter.
    ///
    /// Any text reads as some code; a wrong one matches no pending
    /// pairing, and the door answers that, so this cannot fail.
    #[must_use]
    pub fn read(typed: &str) -> Self {
        Self(B3Hash::digest(canonical(typed).as_bytes()))
    }
}

/// A code as a person typed it, in the one form it was minted in: case,
/// spaces and dashes removed. The pairing handshake carries this form.
pub(crate) fn canonical(typed: &str) -> String {
    typed
        .chars()
        .filter(|each| !each.is_whitespace() && *each != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether `byte` is one of the 32 symbols the canonical text is made of.
pub(crate) fn is_symbol(byte: u8) -> bool {
    ALPHABET.contains(&byte)
}

/// Base32 without padding. Every chunk of five bits becomes one symbol;
/// the last symbol carries the leftover bits in its high end.
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

/// Base32 back to bytes, or `None` at the first character outside the
/// alphabet. Leftover bits at the end are dropped, so several texts read
/// as the same bytes; a caller that accepts only the one canonical text
/// encodes the bytes again and compares.
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
        // RFC 4648 section 10: BASE32("foobar") = "MZXW6YTBOI======".
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
