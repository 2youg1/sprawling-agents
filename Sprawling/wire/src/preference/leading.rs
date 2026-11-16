// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

pub const LINE_HEIGHT_MIN: u32 = 1200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct LineHeight(u32);

impl LineHeight {
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
