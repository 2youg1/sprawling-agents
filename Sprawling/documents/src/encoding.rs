// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

const UTF8_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16_LE_MARK: [u8; 2] = [0xFF, 0xFE];
const UTF16_BE_MARK: [u8; 2] = [0xFE, 0xFF];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Encoding {

    Utf8,

    Utf8Bom,

    Utf16Le,

    Utf16Be,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {

    Text(Encoding),

    Opaque,
}

impl Reading {

    pub fn of(bytes: &[u8]) -> Reading {
        let encoding = Encoding::of_mark(bytes);
        if encoding.spells(bytes) {
            Reading::Text(encoding)
        } else {
            Reading::Opaque
        }
    }
}

impl Encoding {

    pub fn of_mark(front: &[u8]) -> Encoding {
        if front.starts_with(&UTF8_MARK) {
            Encoding::Utf8Bom
        } else if front.starts_with(&UTF16_LE_MARK) {
            Encoding::Utf16Le
        } else if front.starts_with(&UTF16_BE_MARK) {
            Encoding::Utf16Be
        } else {
            Encoding::Utf8
        }
    }

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

    pub fn decode(self, bytes: &[u8]) -> Result<String, AxError> {
        let text = match self {
            Encoding::Utf8 | Encoding::Utf8Bom => std::str::from_utf8(bytes)
                .ok()
                .filter(|text| matches!(self, Encoding::Utf8Bom) || !text.contains('\0'))
                .map(str::to_owned),
            Encoding::Utf16Le | Encoding::Utf16Be => {
                let (units, rest) = bytes.as_chunks::<2>();
                rest.is_empty()
                    .then(|| char::decode_utf16(self.units(units)).collect::<Result<String, _>>())
                    .and_then(Result::ok)
            }
        };
        text.ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a document window",
                format!("{} bytes that are not {self:?} text", bytes.len()),
            )
            .with_recovery("open the document again: its answer says whether it reads as text")
        })
    }

    pub(crate) fn begins_at(self, bytes: &[u8], index: usize, origin: u64) -> bool {
        match self {
            Encoding::Utf8 | Encoding::Utf8Bom => {
                bytes.get(index).is_some_and(|byte| byte & 0xC0 != 0x80)
            }
            Encoding::Utf16Le | Encoding::Utf16Be => {
                let at = origin.saturating_add(crate::span::offset(index));
                let unit = bytes
                    .get(index..)
                    .and_then(<[u8]>::first_chunk::<2>)
                    .map(|pair| self.unit(*pair));
                at.is_multiple_of(2) && unit.is_some_and(|unit| !(0xDC00..=0xDFFF).contains(&unit))
            }
        }
    }

    pub(crate) fn encode(self, text: &str) -> Vec<u8> {
        match self {
            Encoding::Utf8 | Encoding::Utf8Bom => text.as_bytes().to_vec(),
            Encoding::Utf16Le => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            Encoding::Utf16Be => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        }
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
