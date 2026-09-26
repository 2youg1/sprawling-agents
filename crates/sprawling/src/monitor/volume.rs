// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which disk holds the city, and how much room it has left
//! (sprawling-SPEC.md 8-94).

use std::path::Path;

use kernel::degradation::VolumeSpace;
use sysinfo::Disks;

/// Lists the disks once and reads the one that holds `city`.
pub(crate) fn read(city: &Path) -> Option<VolumeSpace> {
    space(&Disks::new_with_refreshed_list(), city)
}

/// The free space and capacity of the disk whose mount point is the
/// longest prefix of `city`; `None` when no disk's mount point is.
pub(crate) fn space(disks: &Disks, city: &Path) -> Option<VolumeSpace> {
    disks
        .list()
        .iter()
        .filter(|disk| city.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .map(|disk| VolumeSpace {
            free_bytes: disk.available_space(),
            total_bytes: disk.total_space(),
        })
}

#[cfg(test)]
#[path = "volume/tests.rs"]
mod tests;
