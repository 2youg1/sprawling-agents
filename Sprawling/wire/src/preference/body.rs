// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

pub const BODY_PX_MIN: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct BodyPx(u32);

impl BodyPx {
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
