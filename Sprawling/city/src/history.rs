// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{AxCode, AxError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum History {
    Absent,
    Present,
}

pub fn has_history(city_root: &Path) -> Result<History, AxError> {
    let dir = kernel::layout::CityLayout::new(city_root).ledger();
    match std::fs::read_dir(&dir) {
        Ok(mut entries) => Ok(match entries.next() {
            Some(_) => History::Present,
            None => History::Absent,
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(History::Absent),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read city history",
            format!("{}: {err}", dir.display()),
        )
        .with_recovery("make the ledger directory readable, or name a different city directory")),
    }
}
