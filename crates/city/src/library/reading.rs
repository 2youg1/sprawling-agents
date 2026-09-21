// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How one shelf becomes holdings: the city's two, and the ones it
//! mounts from elsewhere on this machine.
//!
//! One map keyed by name for every shelf, filled farthest first, so
//! that the nearer shelf replacing the farther one is the order of
//! these calls rather than a reader's judgement.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, B3Hash};

use super::ShelfKey;
use super::shelf::{Holding, Shelf};

/// What a skill on a shelf outside the city is filed as: one directory
/// per skill, holding this file. It is the layout pi, claude and agents
/// all use, and the one place this city states it.
const SKILL_FILE: &str = "SKILL.md";

/// One of the two shelves the city keeps itself: the two a holding can
/// be at an address on.
#[derive(Debug, Clone, Copy)]
pub(super) enum OwnShelf {
    Library,
    Building,
}

impl OwnShelf {
    /// Where a document read off this shelf sits.
    fn at(self, addr: Address) -> Shelf {
        match self {
            OwnShelf::Library => Shelf::Library(addr),
            OwnShelf::Building => Shelf::Building(addr),
        }
    }
}

/// Reads one shelf into the map. A shelf that is not there is an empty
/// shelf: most cities start with nothing settled, and most buildings
/// never keep a skill of their own.
///
/// A holding read from a nearer shelf replaces one of the same name
/// read earlier, which is what makes a building's own copy of a skill
/// the one its residents get.
///
/// # Errors
/// Propagates a directory that exists and cannot be read, a holding
/// that cannot be read, a name this machine spells outside Unicode, and
/// a path the city cannot spell as an address.
pub(super) fn shelve(
    city_root: &Path,
    root: &Path,
    shelf: OwnShelf,
    holdings: &mut BTreeMap<ShelfKey, Holding>,
) -> Result<(), AxError> {
    if !root.exists() {
        return Ok(());
    }
    for section in read_dir(root)? {
        if !section.is_dir() {
            continue;
        }
        let section_name = spelled(&section)?;
        for item in read_dir(&section)? {
            if item.is_dir() || item.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let file = spelled(&item)?;
            let Some(name) = file.strip_suffix(".md").map(str::to_owned) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let text = read_holding(&item)?;
            // The address is read back off the path the layout chose,
            // so a shelf that moves cannot leave the addresses of what
            // sits on it pointing at where it used to be.
            let addr = address_of(city_root, &item)?;
            holdings.insert(
                ShelfKey::of(&name),
                Holding {
                    name,
                    section: section_name.clone(),
                    disclosure: first_line(&text),
                    hash: B3Hash::digest(text.as_bytes()),
                    shelf: shelf.at(addr),
                },
            );
        }
    }
    Ok(())
}

/// Reads one skill off a shelf the city does not keep: one directory per
/// skill, holding [`SKILL_FILE`].
///
/// The shelf's layout belongs to the harness that wrote it, and pi,
/// claude and agents all file a skill that one way, so this is the one
/// place this city states it. A directory without that file is not a
/// skill and is passed over rather than reported: an external shelf
/// holds whatever its owner keeps there.
///
/// The section of such a holding is empty - see `Holding::section` -
/// because that tree has no section level to read one from.
///
/// # Errors
/// Refuses a shelf that exists and is not a directory, and propagates a
/// directory that exists and cannot be read and a name this machine
/// spells outside Unicode.
pub(super) fn shelve_external(
    index: u32,
    root: &Path,
    holdings: &mut BTreeMap<ShelfKey, Holding>,
) -> Result<(), AxError> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "mount an external skill shelf",
            format!("{} is not a directory", root.display()),
        )
        .with_recovery(
            "point `[skills] shelves` at a directory, or take that entry out of the list",
        ));
    }
    for item in read_dir(root)? {
        if !item.is_dir() {
            continue;
        }
        let name = spelled(&item)?;
        if name.is_empty() {
            continue;
        }
        let document = item.join(SKILL_FILE);
        if !document.is_file() {
            continue;
        }
        let text = read_holding(&document)?;
        holdings.insert(
            ShelfKey::of(&name),
            Holding {
                name: name.clone(),
                section: String::new(),
                disclosure: first_line(&text),
                hash: B3Hash::digest(text.as_bytes()),
                shelf: Shelf::External {
                    index,
                    path: format!("{name}/{SKILL_FILE}"),
                },
            },
        );
    }
    Ok(())
}

/// One holding's bytes, or the refusal that names the file.
fn read_holding(item: &Path) -> Result<String, AxError> {
    std::fs::read_to_string(item).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read a library holding",
            format!("{}: {err}", item.display()),
        )
        .with_recovery(
            "give this process permission to read the file named above, or \
             take it off the shelf, then scan again",
        )
    })
}

/// How the city addresses a file on one of its shelves.
fn address_of(city_root: &Path, item: &Path) -> Result<Address, AxError> {
    let relative = item.strip_prefix(city_root).map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            "address a library holding",
            format!("{} is outside {}", item.display(), city_root.display()),
        )
        .with_recovery("scan the library of the city the holding sits in")
    })?;
    let mut spelled_out = Vec::new();
    for component in relative.components() {
        spelled_out.push(
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| unspellable(item))?,
        );
    }
    Address::parse(&spelled_out.join("/"))
}

/// The last element of a path, as the city has to be able to write it.
///
/// A name this machine spells outside Unicode is reported rather than
/// dropped: a skill silently absent from every reading room is the one
/// failure a person cannot see from the catalog.
fn spelled(path: &Path) -> Result<String, AxError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| unspellable(path))
}

fn unspellable(path: &Path) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "read the library",
        format!("{} is not named in Unicode", path.display()),
    )
    .with_recovery("rename it to letters, digits and dashes, then scan again")
}

fn read_dir(path: &Path) -> Result<Vec<PathBuf>, AxError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(path).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the library",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "give this process permission to list the directory named above, then \
             scan again",
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the library",
                format!("{}: {err}", path.display()),
            )
            .with_recovery(
                "give this process permission to list the directory named above, \
                 then scan again",
            )
        })?;
        out.push(entry.path());
    }
    out.sort();
    Ok(out)
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .trim_start_matches(['#', ' '])
        .to_owned()
}
