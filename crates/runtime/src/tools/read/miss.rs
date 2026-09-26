// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `read` answers when the file it was asked for will not open.
//!
//! A miss names what is there instead: the entries of the nearest
//! directory that does exist, closest name first, so the next call can
//! be a correct one rather than a guess. The recovery points at
//! `search`, which every building's tool set carries; `exec` is not in
//! City Hall's.

use std::path::Path;

use kernel::{Address, AxCode, AxError};

use crate::tools::chosen_path::real_location;

/// How many entries a miss offers. A bound on what one refusal costs the
/// context window, not on the directory: the closest names come first,
/// so the ones cut are the least likely to be meant.
const NEARBY_CAP: usize = 16;

/// The refusal for a file that did not open, with the entries of the
/// nearest existing directory in `nearby` when the file is not there.
///
/// A file that exists and will not open is storage, and offers nothing:
/// the caller asked for the right name.
pub(super) fn unread(city_root: &Path, asked: &str, path: &Path, err: &std::io::Error) -> AxError {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "std::io::ErrorKind is an upstream open enum; a file that exists and will not open is storage"
    )]
    let (code, nearby) = match err.kind() {
        std::io::ErrorKind::NotFound => (AxCode::InvalidArgs, nearby(city_root, path)),
        _ => (AxCode::StorageFatal, Vec::new()),
    };
    AxError::failure(code, "read", format!("{asked}: {err}"))
        .with_nearby(nearby)
        .with_recovery(
            "pick a path from `nearby` or a name the catalog lists, or find the file \
             with `search`",
        )
}

/// The entries of the deepest directory above where `missing` really
/// lands that exists, inside the city, as city-relative paths a model
/// may read.
///
/// Listing is best effort: a path whose real location will not resolve,
/// or a directory that will not list, offers no candidates, because the
/// refusal the caller needs is the miss itself.
fn nearby(city_root: &Path, missing: &Path) -> Vec<String> {
    let (Ok(city_root), Ok(missing)) = (
        real_location(city_root, "read", "the city root"),
        real_location(missing, "read", "the missing file"),
    ) else {
        return Vec::new();
    };
    let city_root = city_root.as_path();
    let Some(dir) = missing
        .ancestors()
        .skip(1)
        .take_while(|dir| dir.starts_with(city_root))
        .find(|dir| dir.is_dir())
    else {
        return Vec::new();
    };
    let Ok(listing) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let prefix = if dir == city_root {
        None
    } else {
        let Some(spelled) = spelled(city_root, dir) else {
            return Vec::new();
        };
        Some(spelled)
    };
    let wanted = missing
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut found: Vec<(usize, String)> = listing
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter_map(|name| {
            let path = prefix
                .as_ref()
                .map_or_else(|| name.clone(), |dir| format!("{dir}/{name}"));
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

/// `dir` relative to the city root, spelled with `/`; `None` for a
/// directory named outside Unicode.
fn spelled(city_root: &Path, dir: &Path) -> Option<String> {
    let segments: Option<Vec<&str>> = dir
        .strip_prefix(city_root)
        .ok()?
        .components()
        .map(|component| component.as_os_str().to_str())
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
