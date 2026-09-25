// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The note a person keeps for one model, and where the city keeps it
//! (sprawling-SPEC 8-84).
//!
//! One place, in the city's reserved subtree: a directory at the city
//! root would share its name space with the buildings, and a resident
//! cannot write under the reserved prefix, so the model a note governs
//! cannot rewrite it.

use std::path::Path;

use kernel::{Address, AxCode, AxError, RESERVED_PREFIX};

/// The directory under the reserved prefix that holds one directory per
/// provider, each holding one `<model>.md` per model.
pub(super) const MODELS_DIR: &str = "models";

/// One model's note: the bytes, and the address a reader opens them by.
pub(super) struct ModelNote {
    pub(super) at: Address,
    pub(super) bytes: Vec<u8>,
}

/// The note for `model` on the endpoint named `provider`, when the city
/// has one.
///
/// The location is spelled as an address before it becomes a path. A
/// model id the address grammar refuses names no note in this city, and
/// is answered as a note that is not there; spelling the path first
/// would let an id with a `..` segment read a file outside the reserved
/// subtree.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path for a note that is there and will
/// not read: the person wrote it for this model, and a run sent without
/// it would follow a prompt nobody chose, with nobody told.
pub(super) fn model_note(
    city_root: &Path,
    provider: &str,
    model: &str,
) -> Result<Option<ModelNote>, AxError> {
    let Ok(at) = Address::parse(&format!(
        "{RESERVED_PREFIX}/{MODELS_DIR}/{provider}/{model}.md"
    )) else {
        return Ok(None);
    };
    let path = city_root.join(at.as_str());
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(ModelNote { at, bytes })),
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
