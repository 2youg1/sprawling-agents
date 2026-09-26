// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether a directory already carries a city's history (city-SPEC.md
//! section 8-29).
//!
//! A fact about the ledger directory on disk, so it lives beside the
//! other readers of a city's layout rather than in the assembly point:
//! `init`, `up`, and the doctor all ask it, and the doctor is a reader
//! that must not depend on the module that assembles the city.

use std::path::Path;

use kernel::{AxCode, AxError};

/// Whether a directory carries a city's history.
///
/// Two answers, so a caller cannot read a third state into a `false`: a
/// ledger directory that is missing or empty is `Absent`, and one with
/// an entry is `Present`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum History {
    Absent,
    Present,
}

/// Whether this directory already carries a city's history.
///
/// The one fact `init` refuses on and `up` branches on, read from one
/// place so the two can never disagree about what counts as a city.
///
/// # Errors
/// `StorageFatal` when the ledger directory is there but cannot be
/// listed: calling that `Absent` would let `init` write a second genesis
/// over a city it merely failed to read.
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
