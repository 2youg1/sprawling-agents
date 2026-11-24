// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use base64::Engine as _;
use kernel::{AxCode, AxError, ImageRef};

const MAX_PICTURES: usize = 4;
const MAX_ENCODED: usize = 2 * 1024 * 1024;
const MAX_SIDE: u32 = 4096;

pub(in crate::worker) struct SelectedPicture {
    pub(in crate::worker) reference: ImageRef,
    bytes: Vec<u8>,
}

pub(in crate::worker) fn no_picture_input() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "send a picture",
        "this target only accepts text",
    )
    .with_recovery(
        "choose a model with image input, or send the text alone after removing the image",
    )
}

pub(in crate::worker) fn read_images(images: &[String]) -> Result<Vec<SelectedPicture>, AxError> {
    let bad = |subject| {
        AxError::failure(AxCode::InvalidArgs, "prepare selected images", subject)
            .with_recovery("select up to four PNG crops with at most 4096 pixels per side and 2 MiB after base64 encoding")
    };
    if images.len() > MAX_PICTURES {
        return Err(bad("too many selected images"));
    }
    images
        .iter()
        .map(|image| {
            if image.len() > MAX_ENCODED {
                return Err(bad("a selected image exceeds the encoded size limit"));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(image)
                .map_err(|_| bad("the selected image body is not base64"))?;
            let reference = runtime::png_picture(&bytes)?;
            if reference.width > MAX_SIDE || reference.height > MAX_SIDE {
                return Err(bad("a selected image exceeds the pixel size limit"));
            }
            Ok(SelectedPicture { reference, bytes })
        })
        .collect()
}

pub(in crate::worker) fn store_images(
    store: &mut storage::Cas,
    origin: &storage::BlockOrigin,
    images: &mut [SelectedPicture],
) -> Result<(), AxError> {
    for image in images {
        store
            .put_for(&image.bytes, origin)
            .map_err(storage::StorageError::into_ax)?;
        image.bytes = Vec::new();
    }
    Ok(())
}
