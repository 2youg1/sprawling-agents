// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxCode, AxError};

pub(super) struct ModelNote {
    pub(super) at: Address,
    pub(super) bytes: Vec<u8>,
}

pub(super) fn model_note(
    city_root: &Path,
    building: &Address,
    provider: &str,
    model: &str,
) -> Result<Option<ModelNote>, AxError> {
    for layer in [Some(building), None] {
        let Some(at) = city::model_note_at(layer, provider, model) else {
            return Ok(None);
        };
        if let Some(bytes) = read_note(&city_root.join(at.as_str()))? {
            return Ok(Some(ModelNote { at, bytes }));
        }
    }
    Ok(None)
}

fn read_note(path: &Path) -> Result<Option<Vec<u8>>, AxError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the note for this model",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "every run on this model is sent that file; make it readable, or take it \
             away to send the prompt without it",
        )),
    }
}
