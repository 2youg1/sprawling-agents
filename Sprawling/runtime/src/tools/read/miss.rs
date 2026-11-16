// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Read;
use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError};
use same_file::Handle;

use crate::tools::chosen_path::{Located, real_location, still_judged};
use crate::tools::stamp::Stamp;

pub(super) enum Floor {

    Document,

    Directory { dir: PathBuf, named: String },
}

impl Floor {

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

const NEARBY_CAP: usize = 16;

pub(super) fn text_at(
    asked: &str,
    at: Located,
    floor: &Floor,
) -> Result<(String, Stamp, Stamp), AxError> {
    match at {
        Located::Present(path) if path.is_dir() => Err(AxError::failure(
            AxCode::InvalidArgs,
            "read",
            format!("{asked} is a directory"),
        )
        .with_nearby(entries(&path, floor, ""))
        .with_recovery("read one of the entries in `nearby`, or find a file with `search`")),
        Located::Present(path) => {
            let mut opened = std::fs::File::open(&path)
                .and_then(Handle::from_file)
                .map_err(|err| unread(asked, &path, floor, &err))?;
            still_judged(asked, &path, &opened, "read")?;

            let stamp = |file: &std::fs::File| {
                Stamp::of_file(file).map_err(|err| unread(asked, &path, floor, &err))
            };
            let before = stamp(opened.as_file())?;
            let mut text = String::new();
            opened
                .as_file_mut()
                .read_to_string(&mut text)
                .map_err(|err| unread(asked, &path, floor, &err))?;
            let after = stamp(opened.as_file())?;
            Ok((text, before, after))
        }
        Located::Absent(path) => Err(unread(
            asked,
            &path,
            floor,
            &std::io::ErrorKind::NotFound.into(),
        )),
    }
}

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

fn nearby(missing: &Path, floor: &Floor) -> Vec<String> {
    let Floor::Directory { dir: top, .. } = floor else {
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
    let wanted = missing
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    entries(dir, floor, wanted)
}

fn entries(dir: &Path, floor: &Floor, wanted: &str) -> Vec<String> {
    let Floor::Directory { dir: top, named } = floor else {
        return Vec::new();
    };
    if !dir.starts_with(top) {
        return Vec::new();
    }
    let (Ok(listing), Some(prefix)) = (std::fs::read_dir(dir), spelled(top, named, dir)) else {
        return Vec::new();
    };
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

fn shared_prefix(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(x, y)| x.eq_ignore_ascii_case(y))
        .count()
}
