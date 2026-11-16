// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The space a person asked for between letters
//! (`crates/wire/spec/Preference.lean` D126).
//!
//! **The floor is a type, and there is no ceiling.** Tracking is added
//! to every advance width, so a value under [`TRACKING_MIN`] closes the
//! gap between one letter and the next until they touch: that is a page
//! nobody can read rather than a taste, and it is refused where it is
//! read. Above the floor a larger number only sets the letters further
//! apart, so there is no ceiling.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// The tightest tracking a person may ask for, in thousandths of an em:
/// -50 is five hundredths of an em removed from every letter.
pub const TRACKING_MIN: i32 = -50;

/// A tracking of at least [`TRACKING_MIN`] thousandths of an em.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "i32", into = "i32")]
pub struct Tracking(i32);

impl Tracking {
    /// Sole constructor.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a tracking under [`TRACKING_MIN`].
    pub fn new(per_mille: i32) -> Result<Self, AxError> {
        if per_mille < TRACKING_MIN {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "set the letter spacing",
                per_mille.to_string(),
            )
            .with_recovery(format!(
                "give a spacing of at least {TRACKING_MIN} thousandths of an em"
            )));
        }
        Ok(Tracking(per_mille))
    }

    #[must_use]
    pub fn per_mille(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for Tracking {
    type Error = AxError;

    fn try_from(per_mille: i32) -> Result<Self, AxError> {
        Tracking::new(per_mille)
    }
}

impl From<Tracking> for i32 {
    fn from(tracking: Tracking) -> i32 {
        tracking.0
    }
}

/// Inlined rather than named, for the reason `LineHeight` is.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for Tracking {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Tracking")
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "integer",
            "format": "int32",
            "minimum": TRACKING_MIN,
        })
    }
}
