// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A file inside a package the reading room admits, asked for as
//! `<name>/<path>`.
//!
//! A person admitted the package when they wrote the reading room, so
//! the files beside its `SKILL.md` open the way that document does:
//! past the read bound and the reserved subtree, but never out of the
//! package directory.

use kernel::layout::SKILL_FILE;
use kernel::{Address, AxCode, AxError};

use crate::catalog::{Catalog, Expansion};

/// Where `<name>/<path>` lands when `name` is a package in the catalog.
///
/// `None` when the first segment is not a package the catalog holds -
/// no such name, a single-document skill, or an entry the catalog
/// speaks itself - so the caller goes on to the ordinary path.
///
/// # Errors
/// Refuses a path inside the package with an empty, `.` or `..`
/// segment, or one spelled with `\` or `:`, rather than normalising it:
/// a normalised path is a different path from the one the caller wrote.
pub(super) fn open_in_package(catalog: &Catalog, asked: &str) -> Option<Result<Address, AxError>> {
    let (name, inside) = asked.split_once('/')?;
    let Some(Expansion::Skill { addr }) = catalog.expand(name) else {
        return None;
    };
    let package = addr.strip_suffix(SKILL_FILE)?.strip_suffix('/')?;
    Some(within(package, inside, asked))
}

fn within(package: &str, inside: &str, asked: &str) -> Result<Address, AxError> {
    let plain = inside
        .split('/')
        .all(|segment| !matches!(segment, "" | "." | "..") && !segment.contains(['\\', ':']));
    if !plain {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "read",
            format!("{asked} does not name a file inside the package"),
        )
        .with_recovery(
            "write the path inside the package as plain segments joined by `/`, \
             with no empty, `.` or `..` segment",
        ));
    }
    Address::parse(&format!("{package}/{inside}"))
}
