// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use super::prepared::unavailable;

use super::lines::buildings_of;

pub(super) fn search_archives(city_root: &Path, needle: &str) -> wire::Answer {
    let Ok(buildings) = buildings_of(city_root) else {
        return unavailable(format!("Archives({needle})"));
    };
    let mut hits = Vec::new();
    let wanted = needle.to_lowercase();
    for building in buildings {
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
    wire::Answer::Archive(wire::ArchiveAnswer {
        needle: needle.to_owned(),
        hits,
    })
}
