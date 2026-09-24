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

    /// Whether a picture of these sides may be handed to a model.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the longer side is past the cap; the
    /// recovery names the cap and the two ways to ask for less.
    pub fn admit(self, width: u32, height: u32) -> Result<(), AxError> {
        let longest = width.max(height);
        if longest <= self.0 {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "hand a screenshot to a model",
            format!("its longest side is {longest} pixels"),
        )
        .with_recovery(format!(
            "this city caps a picture at {} pixels on a side: ask for a smaller `scale`, or \
             photograph the region with `ref`",
            self.0
        )))
    }
}

/// 1920 pixels on the longest side of a screenshot.
///
/// Wider than this is mostly wasted context: a model reads the same page
/// from a smaller picture, and a clipped shot of the region it asked about
/// carries more of that region than a downscaled whole page does. The
/// capture states it, so the scaling happens in the driver.
pub const SHOT_MAX_EDGE_PX: ShotMaxEdge = ShotMaxEdge::new(1920);
