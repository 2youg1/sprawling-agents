// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which disk holds the city, and how much room it has left
//! (sprawling-SPEC.md 8-116).

use std::path::{Component, Path, PathBuf, Prefix};

use kernel::degradation::VolumeSpace;
use sysinfo::Disks;

/// Lists the disks once and reads the one that holds `city`, resolved
/// first: a link would name the disk that holds the link, and no mount
/// point is a prefix of a relative or verbatim path.
pub(crate) fn read(city: &Path) -> Option<VolumeSpace> {
    let city = std::fs::canonicalize(city)
        .or_else(|_| std::path::absolute(city))
        .ok()?;
    space(
        &Disks::new_with_refreshed_list(),
        &without_verbatim_disk(&city),
    )
}

/// `\\?\C:\city` respelled `C:\city`, the spelling mount points carry.
fn without_verbatim_disk(path: &Path) -> PathBuf {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(letter) => {
                let mut plain = PathBuf::from(format!("{}:", char::from(letter)));
                plain.extend(components);
                plain
            }
            Prefix::Verbatim(_)
            | Prefix::VerbatimUNC(..)
            | Prefix::DeviceNS(_)
            | Prefix::UNC(..)
            | Prefix::Disk(_) => path.to_path_buf(),
        },
        Some(
            Component::RootDir | Component::CurDir | Component::ParentDir | Component::Normal(_),
        )
        | None => path.to_path_buf(),
    }
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
