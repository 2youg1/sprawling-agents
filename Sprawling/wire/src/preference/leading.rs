// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The height of one line of running text
//! (`crates/wire/spec/Preference.lean` D126).
//!
//! **The floor is a type, and there is no ceiling.** A line box under
//! [`LINE_HEIGHT_MIN`] shares its edges with the lines beside it, so the
//! glyphs of one line meet the glyphs of the next: that is a page nobody
//! can read rather than a taste, and it is refused where it is read —
//! from the person's `[ui]` file, from a `PutPreferences` frame, and,
//! through the `minimum` the schema carries, from the answer the page
//! reads. Above the floor a larger number only sets the text looser, so
//! there is no ceiling, for the reason `BodyPx` has none.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// The smallest line box a person may ask for, in thousandths of the
/// font size: 1200 is a line one and a fifth times as tall as its text.
///
/// Thousandths rather than hundredths because the leading the
/// stylesheet draws is 1.625, which hundredths cannot spell.
pub const LINE_HEIGHT_MIN: u32 = 1200;

/// A line box of at least [`LINE_HEIGHT_MIN`] thousandths of the font
/// size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct LineHeight(u32);

impl LineHeight {
    /// Sole constructor.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a line box under [`LINE_HEIGHT_MIN`].
    pub fn new(per_mille: u32) -> Result<Self, AxError> {
        if per_mille < LINE_HEIGHT_MIN {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "set the line height",
                per_mille.to_string(),
            )
            .with_recovery(format!(
                "give a height of at least {LINE_HEIGHT_MIN} thousandths of the font size"
            )));
        }
        Ok(LineHeight(per_mille))
    }

    #[must_use]
    pub fn per_mille(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for LineHeight {
    type Error = AxError;

    fn try_from(per_mille: u32) -> Result<Self, AxError> {
        LineHeight::new(per_mille)
    }
}

impl From<LineHeight> for u32 {
    fn from(height: LineHeight) -> u32 {
        height.0
    }
}

/// Inlined rather than named: a named integer becomes a branded type in
/// the generated client, and every place the page writes a height would
/// then have to mint the brand first for no guarantee beyond `minimum`.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for LineHeight {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("LineHeight")
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "integer",
            "format": "uint32",
            "minimum": LINE_HEIGHT_MIN,
        })
    }
}
