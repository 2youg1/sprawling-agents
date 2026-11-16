// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};
use wire::GuideProgress;

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

pub(crate) fn put(city_root: &Path, progress: &GuideProgress) -> Result<(), AxError> {
    let file = CityLayout::new(city_root).guide();
    let text = toml::to_string_pretty(progress).map_err(|err| invalid(&file, &err))?;
    city::edit_document(&file, |held| held.replace(text.as_bytes()))
}

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
