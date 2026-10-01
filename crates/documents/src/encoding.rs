// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which characters a document version's bytes spell
//! (`crates/documents/spec/Encoding.lean`, D4).
//!
//! A byte-order mark decides first, so a UTF-16 file - which holds a NUL
//! beside every ASCII character - is text. Without a mark the bytes are
//! UTF-8 or nothing, and a NUL still says "not text": no text file holds
//! one, and every store the city keeps (a redb file, a CAS blob) does.
//! UTF-16 without a mark is not guessed at, because a guess that is wrong
//! shows a reader characters the file does not hold.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

const UTF8_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16_LE_MARK: [u8; 2] = [0xFF, 0xFE];
const UTF16_BE_MARK: [u8; 2] = [0xFE, 0xFF];

/// The character encoding one document version is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Encoding {
    /// UTF-8 with no byte-order mark and no NUL.
    Utf8,
    /// UTF-8 behind its byte-order mark.
    Utf8Bom,
    /// UTF-16, low byte first, behind its byte-order mark.
    Utf16Le,
    /// UTF-16, high byte first, behind its byte-order mark.
    Utf16Be,
}

/// What one version's bytes are to a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// Text in this encoding, every byte of it.
    Text(Encoding),
    /// Not text in any encoding this city reads: a reader is given the
    /// version and the size, never characters.
    Opaque,
}

impl Reading {
    /// Judges one whole version.
    pub fn of(bytes: &[u8]) -> Reading {
        let _ = bytes;
        Reading::Opaque
    }
}

impl Encoding {
    /// The encoding the byte-order mark at the front of a version names,
    /// and UTF-8 when there is none. Reads at most the first three bytes,
    /// so a window of a stored version learns its encoding from them.
    pub fn of_mark(front: &[u8]) -> Encoding {
        let _ = front;
        Encoding::Utf8
    }

    /// Whether `bytes` - a whole version, or a stretch of one cut at
    /// character boundaries - spell text in this encoding.
    fn spells(self, bytes: &[u8]) -> bool {
        match self {
            Encoding::Utf8 => !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok(),
            Encoding::Utf8Bom => std::str::from_utf8(bytes).is_ok(),
            Encoding::Utf16Le | Encoding::Utf16Be => {
                let (units, rest) = bytes.as_chunks::<2>();
                rest.is_empty() && char::decode_utf16(self.units(units)).all(|unit| unit.is_ok())
            }
        }
    }

    /// The characters a stretch cut at character boundaries spells.
    ///
    /// A mark at the front decodes as U+FEFF, the first character of the
    /// text (D5), so the text encodes back to exactly these bytes.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming the encoding when the bytes do not spell
    /// text in it.
    pub fn decode(self, bytes: &[u8]) -> Result<String, AxError> {
        let _ = (self, bytes);
        Ok(String::new())
    }

    /// Whether a character begins at byte `index` of `bytes`, where
    /// `bytes[0]` is byte `origin` of the version. `index == bytes.len()`
    /// is a beginning only when the caller has said the version ends
    /// there, which is [`crate::window`]'s business, not this one's.
    pub(crate) fn begins_at(self, bytes: &[u8], index: usize, origin: u64) -> bool {
        let _ = (self, bytes, index, origin);
        true
    }

    fn units(self, pairs: &[[u8; 2]]) -> impl Iterator<Item = u16> {
        pairs.iter().map(move |pair| self.unit(*pair))
    }

    fn unit(self, pair: [u8; 2]) -> u16 {
        match self {
            Encoding::Utf16Be => u16::from_be_bytes(pair),
            Encoding::Utf8 | Encoding::Utf8Bom | Encoding::Utf16Le => u16::from_le_bytes(pair),
        }
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
mod tests;
