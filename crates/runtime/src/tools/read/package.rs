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
//! package directory, whether by a segment or through a link.

use std::path::{Path, PathBuf};

use kernel::layout::SKILL_FILE;
use kernel::{Address, AxCode, AxError};

use crate::catalog::{Catalog, Expansion};
use crate::tools::chosen_path::real_location;

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
/// Refuses with `E_GATE_DENIED` a file whose real location, links
/// resolved, lies outside the package directory as the shelf spells it,
/// which also refuses a package directory that is itself a link.
pub(super) fn open_in_package(
    catalog: &Catalog,
    city_root: &Path,
    asked: &str,
) -> Option<Result<PathBuf, AxError>> {
    let (name, inside) = asked.split_once('/')?;
    let Some(Expansion::Skill { addr }) = catalog.expand(name) else {
        return None;
    };
    let package = addr.strip_suffix(SKILL_FILE)?.strip_suffix('/')?;
    Some(
        within(package, inside, asked)
            .and_then(|target| stays_inside(city_root, package, &target, asked)),
    )
}

/// The file's real location, present or absent, provided that lies in
/// the package.
fn stays_inside(
    city_root: &Path,
    package: &str,
    target: &Address,
    asked: &str,
) -> Result<PathBuf, AxError> {
    let real = real_location(&under(city_root, target.as_str()), "read", asked)?;
    let shelf = under(&real_location(city_root, "read", asked)?, package);
    if real.starts_with(&shelf) {
        return Ok(real);
    }
    Err(AxError::failure(
        AxCode::GateDenied,
        "read",
        format!("{asked} leads out of the package through a link"),
    )
    .with_recovery("name a file that lies inside the package itself, not behind a link in it"))
}

fn under(root: &Path, addr: &str) -> PathBuf {
    addr.split('/')
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
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
