// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The size of a line of body text a person asked for
//! (`crates/wire/spec/Preference.lean` D51).
//!
//! **The floor is a type, and there is no ceiling.** A number under
//! [`BODY_PX_MIN`] is refused where it is read — from the person's `[ui]`
//! file, from a `PutPreferences` frame, and, through the `minimum` the
//! schema carries, from the answer the page reads — so the three readers
//! meet one rule instead of three copies of it.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// The smallest body size a person may ask for, in pixels.
///
/// The floor is the smallest size the person ruled the pages offer; the
/// colour gate holds its contrast tiers at the stylesheet's default size,
/// not at this one.
pub const BODY_PX_MIN: u32 = 12;

/// A body size of at least [`BODY_PX_MIN`] pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct BodyPx(u32);

impl BodyPx {
    /// Sole constructor.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a size under [`BODY_PX_MIN`].
    pub fn new(px: u32) -> Result<Self, AxError> {
        if px < BODY_PX_MIN {
            return Err(
                AxError::failure(AxCode::InvalidArgs, "set the body size", px.to_string())
                    .with_recovery(format!(
                        "give a whole number of pixels of at least {BODY_PX_MIN}"
                    )),
            );
        }
        Ok(BodyPx(px))
    }

    #[must_use]
    pub fn px(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for BodyPx {
    type Error = AxError;

    fn try_from(px: u32) -> Result<Self, AxError> {
        BodyPx::new(px)
    }
}

impl From<BodyPx> for u32 {
    fn from(size: BodyPx) -> u32 {
        size.0
    }
}

/// Inlined rather than named: a named integer becomes a branded type in
/// the generated client, and every place the page writes a size would
/// then have to mint the brand first for no guarantee beyond `minimum`.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for BodyPx {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("BodyPx")
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "integer",
            "format": "uint32",
            "minimum": BODY_PX_MIN,
        })
    }
}
