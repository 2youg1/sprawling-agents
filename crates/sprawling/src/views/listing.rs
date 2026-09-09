// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One directory of the city, read at the moment of asking.
//!
//! The disk is the authority, as it is for `read_building`: a listing
//! kept beside the tree would be a second copy of what the tree says. A
//! directory this cannot read lists as empty rather than refusing - on
//! the page, a directory nobody can open is an empty directory, which
//! is the same thing the building page says about a plan it cannot
//! read.

use std::path::Path;

use kernel::Address;

use super::holding::Views;

impl Views {
    /// One level of the tree under `at`, or the city root when `at` is
    /// absent: directories first, then files, each in name order.
    pub(super) fn listing_answer(&self, at: Option<&Address>) -> channels::ListingAnswer {
        channels::ListingAnswer {
            at: at.cloned(),
            entries: list(&resolve(&self.city_root, at)),
        }
    }
}

/// Where an address points inside this city.
pub(super) fn resolve(city_root: &Path, at: Option<&Address>) -> std::path::PathBuf {
    match at {
        Some(addr) => city_root.join(addr.as_str()),
        None => city_root.to_path_buf(),
    }
}

fn list(dir: &Path) -> Vec<channels::Entry> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // `metadata` follows a link, so a link to a directory is a
        // directory to a person walking the tree.
        let Ok(meta) = std::fs::metadata(entry.path()) else {
            continue;
        };
        if meta.is_dir() {
            directories.push(channels::Entry {
                name,
                kind: channels::EntryKind::Directory,
            });
        } else {
            files.push(channels::Entry {
                name,
                kind: channels::EntryKind::File { bytes: meta.len() },
            });
        }
    }
    directories.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));
    directories.extend(files);
    directories
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn names(entries: &[channels::Entry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    /// The city root lists its reserved subtree beside its buildings:
    /// both are the design this view exists to show.
    #[test]
    fn the_city_root_lists_the_reserved_subtree_and_the_buildings() {
        let dir = tempfile::tempdir().unwrap();
        crate::assembly::init_city(dir.path()).unwrap();
        let mut views = Views::new(dir.path());
        let channels::Answer::Listing(answer) =
            views.answer(&channels::Query::Listing { at: None })
        else {
            panic!("Listing answers with a listing");
        };
        assert_eq!(answer.at, None);
        let listed = names(&answer.entries);
        assert!(listed.contains(&".sprawling"), "{listed:?}");
        assert!(listed.contains(&"hall"), "{listed:?}");
        // The reserved subtree, City Hall, and the one file a city keeps
        // at its root - in that order, because directories come first.
        assert_eq!(listed, vec![".sprawling", "hall", "City.md"]);
    }

    /// Directories come before files, so a tree reads the same way at
    /// every level.
    #[test]
    fn a_building_lists_its_directories_before_its_files() {
        let dir = tempfile::tempdir().unwrap();
        crate::assembly::init_city(dir.path()).unwrap();
        let mut views = Views::new(dir.path());
        let hall = Address::parse("hall").unwrap();
        let channels::Answer::Listing(answer) =
            views.answer(&channels::Query::Listing { at: Some(hall) })
        else {
            panic!("Listing answers with a listing");
        };
        let first_file = answer
            .entries
            .iter()
            .position(|entry| matches!(entry.kind, channels::EntryKind::File { .. }))
            .expect("a building has files at its root");
        assert!(
            answer.entries[..first_file]
                .iter()
                .all(|entry| entry.kind == channels::EntryKind::Directory),
            "{:?}",
            names(&answer.entries)
        );
        assert!(names(&answer.entries).contains(&"Roadmap.md"));
        let roadmap = answer
            .entries
            .iter()
            .find(|entry| entry.name == "Roadmap.md")
            .unwrap();
        assert!(matches!(roadmap.kind, channels::EntryKind::File { bytes } if bytes > 0));
    }

    /// A directory nobody can open is an empty directory on the page.
    #[test]
    fn a_missing_directory_lists_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let nowhere = Address::parse("nowhere/at/all").unwrap();
        let channels::Answer::Listing(answer) = views.answer(&channels::Query::Listing {
            at: Some(nowhere.clone()),
        }) else {
            panic!("Listing answers with a listing");
        };
        assert!(answer.entries.is_empty());
        assert_eq!(answer.at, Some(nowhere));
    }
}
