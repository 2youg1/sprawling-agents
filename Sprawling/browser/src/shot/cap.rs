// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShotMaxEdge(u32);

impl ShotMaxEdge {
    #[must_use]
    pub(crate) const fn new(px: u32) -> ShotMaxEdge {
        ShotMaxEdge(px)
    }

    #[must_use]
    pub(crate) fn image_size(self) -> Value {
        let px = self.0;
        json!({ "maxWidth": px, "maxHeight": px })
    }

    #[must_use]
    pub fn exceeds(self, width: u32, height: u32) -> bool {
        width.max(height) > self.0
    }

    pub(crate) fn fit(self, longest: u32, requested_percent: u32) -> Result<Value, AxError> {
        let side = u64::from(longest.max(1));
        let asked = u64::from(requested_percent.max(1));
        let within = u64::from(self.0)
            .checked_mul(10_000)
            .and_then(|n| n.checked_div(side))
            .ok_or_else(unstatable)?;
        let ratio = within
            .checked_mul(asked)
            .and_then(|n| n.checked_div(100))
            .ok_or_else(unstatable)?;
        let ratio = ratio.min(asked.checked_mul(100).ok_or_else(unstatable)?);
        if ratio == 0 {
            return Err(unstatable());
        }
        let q4 = u32::try_from(ratio).map_err(|_| unstatable())?;
        let whole = q4.checked_div(10_000).ok_or_else(unstatable)?;
        let rest = q4.checked_rem(10_000).ok_or_else(unstatable)?;
        serde_json::from_str(&format!("{whole}.{rest:04}")).map_err(|_| unstatable())
    }

    pub fn admit(self, width: u32, height: u32) -> Result<(), AxError> {
        if !self.exceeds(width, height) {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "hand a screenshot to a model",
            format!("its longest side is {} pixels", width.max(height)),
        )
        .with_recovery(format!(
            "this city caps a picture at {} pixels on a side, and this engine would not go \
             below it: ask for a smaller `scale`, or photograph the region with `ref`",
            self.0
        )))
    }
}

fn unstatable() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "photograph a page",
        "the density that would fit it is not a number this build can state",
    )
    .with_recovery("report this against browser::shot::cap: the ratio is built from integers")
}

pub const SHOT_MAX_EDGE_PX: ShotMaxEdge = ShotMaxEdge::new(1920);
