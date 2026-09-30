// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What every building has archived, searched across the shelves at the
//! moment of asking.
//!
//! The files are the authority, for the reason `BuildingView` gives: an
//! index kept beside them would be a second copy of what the disk says,
//! and the one that drifted would be the one nobody was looking at.

use std::path::Path;

use super::lines::buildings_of;

/// Every archive entry whose subject contains `needle`, across every
/// building, read from the shelves at the moment of asking.
pub(super) fn search_archives(city_root: &Path, needle: &str) -> wire::ArchiveAnswer {
    let mut hits = Vec::new();
    let wanted = needle.to_lowercase();
    for building in buildings_of(city_root) {
        let Ok(entries) = city::archive_index(city_root, &building) else {
            continue;
        };
        for entry in entries {
            if !entry.subject.to_lowercase().contains(&wanted) {
                continue;
            }
            hits.push(wire::ArchiveHit {
                building: building.clone(),
                kind: entry.kind.as_str().to_owned(),
                day: entry.day,
                subject: entry.subject,
            });
        }
    }
    wire::ArchiveAnswer {
        needle: needle.to_owned(),
        hits,
    }
}
