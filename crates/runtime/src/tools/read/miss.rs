// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Opening the place a read landed on, and what `read` answers when
//! the file it was asked for will not open.
//!
//! A miss names what is there instead: the entries of the nearest
//! directory that does exist, closest name first, so the next call can
//! be a correct one rather than a guess. It never lists above the
//! directory the call was admitted to, because what lies above it is
//! not something this call was allowed to see. The recovery points at
//! `search`, which every building's tool set carries; `exec` is not in
//! City Hall's.

use std::io::Read;
use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError};
use same_file::Handle;

use crate::tools::chosen_path::{Located, real_location};

/// The highest directory a miss may list.
pub(super) enum Floor {
    /// A catalog entry that names one document: there is no directory
    /// the entry admitted, so a miss offers nothing.
    Document,
    /// The directory the call was admitted to, as it really lies, and
    /// the name the caller reaches it by: the first segment of a city
    /// path, or a package's catalog name.
    Directory { dir: PathBuf, named: String },
}

impl Floor {
    /// The floor of a path the model chose: its first segment, so a miss
    /// never climbs to the city root and lists the buildings there.
    pub(super) fn of_address(city_root: &Path, addr: &Address) -> Floor {
        let Some(first) = addr.as_str().split('/').next() else {
            return Floor::Document;
        };
        real_location(&city_root.join(first), "read", first).map_or(Floor::Document, |dir| {
            Floor::Directory {
                dir: dir.into_path(),
                named: first.to_owned(),
            }
        })
    }
}

/// How many entries a miss offers. A bound on what one refusal costs the
/// context window, not on the directory: the closest names come first,
/// so the ones cut are the least likely to be meant.
const NEARBY_CAP: usize = 16;

/// The text at `at`, or the miss. A location that was absent when it
/// was judged is reported missing without being opened, so a link
/// placed there since cannot lead the open past the judgement. A
/// present one is read from the handle the open returned, and only
/// after [`still_judged`] has found that handle is the file judged.
pub(super) fn text_at(asked: &str, at: Located, floor: &Floor) -> Result<String, AxError> {
    match at {
        Located::Present(path) => {
            let mut opened = std::fs::File::open(&path)
                .and_then(Handle::from_file)
                .map_err(|err| unread(asked, &path, floor, &err))?;
            still_judged(asked, &path, &opened)?;
            let mut text = String::new();
            opened
                .as_file_mut()
                .read_to_string(&mut text)
                .map_err(|err| unread(asked, &path, floor, &err))?;
            Ok(text)
        }
        Located::Absent(path) => Err(unread(
            asked,
            &path,
            floor,
            &std::io::ErrorKind::NotFound.into(),
        )),
    }
}

/// Whether the file `opened` at the judged real location `judged` is
/// still the file judged there: the location must still resolve to
/// itself, so no link was swapped onto it after the judgement, and the
/// file at it now must be the one opened, so no link was there during
/// the open and gone again before the resolution. The refusal does not
/// say where a swapped link leads.
fn still_judged(asked: &str, judged: &Path, opened: &Handle) -> Result<(), AxError> {
    let changed = |why: &str| {
        AxError::failure(
            AxCode::GateDenied,
            "read",
            format!("{asked} changed after it was judged: {why}"),
        )
        .with_recovery("read it again once nothing is moving the directories on its path")
    };
    match std::fs::canonicalize(judged) {
        Ok(real) if real == judged => {}
        Ok(_) => return Err(changed("a link now stands on its path")),
        Err(err) => return Err(changed(&err.to_string())),
    }
    match Handle::from_path(judged) {
        Ok(now) if now == *opened => Ok(()),
        Ok(_) => Err(changed("another file stands there now")),
        Err(err) => Err(changed(&err.to_string())),
    }
}

/// The refusal for a file that did not open, with the entries of the
/// nearest existing directory in `nearby` when the file is not there.
///
/// A file that exists and will not open is storage, and offers nothing:
/// the caller asked for the right name.
fn unread(asked: &str, path: &Path, floor: &Floor, err: &std::io::Error) -> AxError {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "std::io::ErrorKind is an upstream open enum; a file that exists and will not open is storage"
    )]
    let (code, nearby) = match err.kind() {
        std::io::ErrorKind::NotFound => (AxCode::InvalidArgs, nearby(path, floor)),
        _ => (AxCode::StorageFatal, Vec::new()),
    };
    AxError::failure(code, "read", format!("{asked}: {err}"))
        .with_nearby(nearby)
        .with_recovery(
            "pick a path from `nearby` or a name the catalog lists, or find the file \
             with `search`",
        )
}

/// The entries of the deepest existing directory above `missing`, a
/// real location, and no higher than `floor`, spelled the way the
/// caller reaches them.
///
/// Listing is best effort: a directory that will not list offers no
/// candidates, because the refusal the caller needs is the miss itself.
fn nearby(missing: &Path, floor: &Floor) -> Vec<String> {
    let Floor::Directory { dir: top, named } = floor else {
        return Vec::new();
    };
    let Some(dir) = missing
        .ancestors()
        .skip(1)
        .take_while(|dir| dir.starts_with(top))
        .find(|dir| dir.is_dir())
    else {
        return Vec::new();
    };
    let (Ok(listing), Some(prefix)) = (std::fs::read_dir(dir), spelled(top, named, dir)) else {
        return Vec::new();
    };
    let wanted = missing
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut found: Vec<(usize, String)> = listing
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter_map(|name| {
            let path = format!("{prefix}/{name}");
            Address::parse(&path)
                .is_ok_and(|addr| !addr.is_reserved())
                .then(|| (shared_prefix(wanted, &name), path))
        })
        .collect();
    found.sort_by(|(a_len, a), (b_len, b)| b_len.cmp(a_len).then_with(|| a.cmp(b)));
    found
        .into_iter()
        .take(NEARBY_CAP)
        .map(|(_, path)| path)
        .collect()
}

/// `dir` as the caller names it: `named`, then the segments from `top`
/// down to `dir`; `None` for a directory named outside Unicode.
fn spelled(top: &Path, named: &str, dir: &Path) -> Option<String> {
    let segments: Option<Vec<&str>> = std::iter::once(Some(named))
        .chain(
            dir.strip_prefix(top)
                .ok()?
                .components()
                .map(|component| component.as_os_str().to_str()),
        )
        .collect();
    segments.map(|segments| segments.join("/"))
}

/// How many leading characters two names share, ignoring case: the
/// closeness a misspelt extension or a wrong capital keeps.
fn shared_prefix(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(x, y)| x.eq_ignore_ascii_case(y))
        .count()
}
