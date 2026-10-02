// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How far the person has got through this city's first-run guide
//! (`crates/accounting/spec/Guide.lean` §8-18-2, `crates/wire/Spec.lean` §8-68).
//!
//! **One record, one grammar.** The file is `wire::GuideProgress`
//! serialised, so the keys the file may hold and the fields the page
//! reads are one declaration: this module reads and writes, and says
//! nothing about what a step or a mark is.
//!
//! **Whole, and the later write stays.** A write replaces the file
//! through `city::document`, so a reader during it finds the progress
//! before or after it. Two browsers moving the guide at once keep the
//! one that wrote last: progress is where a page stands, and a baseline
//! guard would refuse an ordinary step forward.

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};
use wire::GuideProgress;

/// The progress this city's file states, and the guide at its start for
/// a city nobody has guided.
///
/// # Errors
/// `E_CONFIG_INVALID` for a file this build does not read, and
/// `E_STORAGE_FATAL` for one that cannot be read.
pub(crate) fn read(city_root: &Path) -> Result<GuideProgress, AxError> {
    let file = CityLayout::new(city_root).guide();
    match std::fs::read_to_string(&file) {
        Ok(text) => toml::from_str(&text).map_err(|err| invalid(&file, &err)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(GuideProgress::default()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the first-run guide's progress",
            format!("{}: {err}", file.display()),
        )
        .with_recovery("fix that file's permissions, or delete it to start the guide again")),
    }
}

/// Replaces this city's progress with `progress`, whole.
///
/// # Errors
/// `E_CONFIG_INVALID` for progress TOML will not write, and
/// `E_STORAGE_FATAL` for a file that cannot be replaced.
pub(crate) fn put(city_root: &Path, progress: &GuideProgress) -> Result<(), AxError> {
    let file = CityLayout::new(city_root).guide();
    let text = toml::to_string_pretty(progress).map_err(|err| invalid(&file, &err))?;
    city::edit_document(&file, |held| held.replace(text.as_bytes()))
}

/// One refusal for a file that is not the record it should be. The keys
/// are not listed here: `GuideProgress` states them.
fn invalid(file: &Path, why: &impl std::fmt::Display) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read the first-run guide's progress",
        format!("{}: {why}", file.display()),
    )
    .with_recovery(
        "delete the file to start the guide again; it holds where the guide stands, \
         not anything the city is configured with",
    )
}
