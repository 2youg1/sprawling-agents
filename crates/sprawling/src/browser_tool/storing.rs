// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A screenshot, put where it becomes evidence.
//!
//! The picture is the one thing this tool returns that a model cannot be
//! told in words, so the bytes go into the content store and the locator
//! travels instead. The cap on either side is judged here, on the sides
//! read from the bytes, and a picture past it is asked for once more at
//! the density that fits it: a capture the city cannot hand over is worse
//! than a capture taken twice.

use kernel::{AxError, ImageRef, Locator, ToolOutcome};
use serde_json::Value;

use super::Browser;
use super::answering::payload;
use browser::{SHOT_MAX_EDGE_PX, Shot, ShotRequest};

impl Browser {
    /// A screenshot, put where it becomes evidence.
    ///
    /// Three things happen together or none does: the bytes land in the
    /// content store, the payload carries the locator and the two sides,
    /// and the outcome carries the picture itself so the model sees it.
    /// A picture past the cap is asked for once more at the density that
    /// fits it, because the cap has to bind without refusing the shot —
    /// and the bytes the admission reads are the second capture's.
    ///
    /// # Errors
    /// Propagates a reply this build cannot read, a content store that
    /// will not take the bytes, and a driver that will not go below the
    /// cap no matter what density it is asked for.
    pub(super) fn stored(
        &mut self,
        request: &ShotRequest,
        result: &Value,
    ) -> Result<ToolOutcome, AxError> {
        let shot = Shot::read(result, request.format())?;
        SHOT_MAX_EDGE_PX.admit(shot.width(), shot.height())?;
        let hash = self
            .cas
            .put_for(shot.bytes(), &self.origin)
            .map_err(memory::MemoryError::into_ax)?;
        let locator = Locator::cas(hash);
        let picture = ImageRef {
            locator,
            media_type: shot.media(),
            width: shot.width(),
            height: shot.height(),
        };
        Ok(ToolOutcome {
            result: payload(vec![
                ("image", Value::String(picture.locator.to_string())),
                ("width", Value::from(picture.width)),
                ("height", Value::from(picture.height)),
                (
                    "media_type",
                    Value::String(picture.media_type.mime().to_owned()),
                ),
            ])?,
            attachments: vec![picture],
        })
    }
}
