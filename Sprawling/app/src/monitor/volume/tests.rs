// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, reason = "test code")]

use super::*;

#[test]
fn a_relative_city_path_reads_the_disk_that_holds_it() {
    let here = std::env::current_dir().unwrap();
    let total = |city: &Path| read(city).map(|space| space.total_bytes);
    assert!(total(&here).is_some(), "the working directory is on a disk");
    assert_eq!(total(Path::new(".")), total(&here));
}

#[test]
fn a_canonical_city_path_reads_the_disk_that_holds_it() {
    let here = std::env::current_dir().unwrap();
    let canonical = std::fs::canonicalize(&here).unwrap();
    let total = |city: &Path| read(city).map(|space| space.total_bytes);
    assert!(total(&here).is_some(), "the working directory is on a disk");
    assert_eq!(total(&canonical), total(&here));
}
