// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The cap on a picture

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

/// The most either side of a picture this city hands a model may be.
///
/// One figure and two readers, which are the two ends of one thing: the
/// capture states it as `imageSize`, and the reply is judged against it on
/// the sides read from the bytes. A second spelling would let the two
/// disagree, and the one that would win is the request's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShotMaxEdge(u32);

impl ShotMaxEdge {
    /// The only way a value of this type exists; the policy instance is
    /// [`SHOT_MAX_EDGE_PX`].
    #[must_use]
    pub(crate) const fn new(px: u32) -> ShotMaxEdge {
        ShotMaxEdge(px)
    }

    /// The cap as a capture states it, so the driver scales before the
    /// bytes exist and no page-visible fact is touched to save them.
    #[must_use]
    pub(crate) fn image_size(self) -> Value {
        let px = self.0;
        json!({ "maxWidth": px, "maxHeight": px })
    }

    /// Whether a picture of these sides is past the cap.
    ///
    /// The one comparison the cap makes, read by the capture that has to
    /// ask again and by the admission that has the final word.
    #[must_use]
    pub fn exceeds(self, width: u32, height: u32) -> bool {
        width.max(height) > self.0
    }

    /// The density that brings a picture of these sides inside the cap,
    /// never above the density the caller asked for.
    ///
    /// `longest` is the side read from the bytes of a capture that came
    /// back too big; `requested_percent` is what the caller asked for,
    /// 100 when nobody said. The answer is a JSON number built from
    /// digits, so no float exists here either.
    ///
    /// # Errors
    /// Refuses a ratio this build cannot state, which a side long enough
    /// to round the density to zero reaches.
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

    /// Whether a picture of these sides may be handed to a model.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the longer side is past the cap; the
    /// recovery names the cap and the two ways to ask for less.
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

/// A ratio this build cannot state, which is a defect rather than a
/// page's answer.
fn unstatable() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "photograph a page",
        "the density that would fit it is not a number this build can state",
    )
    .with_recovery("report this against browser::shot::cap: the ratio is built from integers")
}

/// 1920 pixels on the longest side of a screenshot.
///
/// Wider than this is mostly wasted context: a model reads the same page
/// from a smaller picture, and a clipped shot of the region it asked about
/// carries more of that region than a downscaled whole page does. The
/// capture states it, so the scaling happens in the driver.
pub const SHOT_MAX_EDGE_PX: ShotMaxEdge = ShotMaxEdge::new(1920);
