// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, ImageRef, Locator, ToolOutcome};
use serde_json::Value;

use super::Browser;
use super::answering::payload;
use browser::{SHOT_MAX_EDGE_PX, Shot, ShotRequest};

impl Browser {
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
            .map_err(storage::StorageError::into_ax)?;
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
