// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::Address;

pub(super) fn listing_answer(city_root: &Path, at: Option<Address>) -> wire::ListingAnswer {
    wire::ListingAnswer {
        entries: list(&resolve(city_root, at.as_ref())),
        at,
    }
}

pub(super) fn resolve(city_root: &Path, at: Option<&Address>) -> std::path::PathBuf {
    match at {
        Some(addr) => city_root.join(addr.as_str()),
        None => city_root.to_path_buf(),
    }
}

fn list(dir: &Path) -> Vec<wire::Entry> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();

        let Ok(meta) = std::fs::metadata(entry.path()) else {
            continue;
        };
        if meta.is_dir() {
            directories.push(wire::Entry {
                name,
                kind: wire::EntryKind::Directory,
            });
        } else {
            files.push(wire::Entry {
                name,
                kind: wire::EntryKind::File { bytes: meta.len() },
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
    use crate::views::Views;

    fn names(entries: &[wire::Entry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    #[test]
    fn the_city_root_lists_the_reserved_subtree_and_the_buildings() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let mut views = Views::new(dir.path());
        let wire::Answer::Listing(answer) = views.answer(&wire::Query::Listing { at: None }) else {
            panic!("Listing answers with a listing");
        };
        assert_eq!(answer.at, None);
        let listed = names(&answer.entries);
        assert!(listed.contains(&".sprawling"), "{listed:?}");
        assert!(listed.contains(&"hall"), "{listed:?}");

        assert_eq!(
            listed,
            vec![".git", ".sprawling", "hall", ".gitignore", "City.md"]
        );
    }

    #[test]
    fn a_building_lists_its_directories_before_its_files() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let mut views = Views::new(dir.path());
        let hall = Address::parse("hall").unwrap();
        let wire::Answer::Listing(answer) = views.answer(&wire::Query::Listing { at: Some(hall) })
        else {
            panic!("Listing answers with a listing");
        };
        let first_file = answer
            .entries
            .iter()
            .position(|entry| matches!(entry.kind, wire::EntryKind::File { .. }))
            .expect("a building has files at its root");
        assert!(
            answer.entries[..first_file]
                .iter()
                .all(|entry| entry.kind == wire::EntryKind::Directory),
            "{:?}",
            names(&answer.entries)
        );
        assert!(names(&answer.entries).contains(&"Roadmap.md"));
        let roadmap = answer
            .entries
            .iter()
            .find(|entry| entry.name == "Roadmap.md")
            .unwrap();
        assert!(matches!(roadmap.kind, wire::EntryKind::File { bytes } if bytes > 0));
    }

    #[test]
    fn a_missing_directory_lists_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let nowhere = Address::parse("nowhere/at/all").unwrap();
        let wire::Answer::Listing(answer) = views.answer(&wire::Query::Listing {
            at: Some(nowhere.clone()),
        }) else {
            panic!("Listing answers with a listing");
        };
        assert!(answer.entries.is_empty());
        assert_eq!(answer.at, Some(nowhere));
    }
}
