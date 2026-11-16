// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, B3Hash};

use super::install::{Installed, Slot, install};

pub const SHIPPED_SECTION: &str = "shipped";

const STAGING_DIR: &str = ".shipped";

const SHIPPED: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/shipped_skills.rs"));

pub fn shelve_shipped(
    city_root: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
) -> Result<Vec<Installed>, AxError> {
    let staging = CityLayout::new(city_root).library().join(STAGING_DIR);
    clear(&staging)?;
    let shelved = stage(&staging).and_then(|names| {
        let slot = Slot::library(SHIPPED_SECTION)?;
        names
            .into_iter()
            .map(|name| install(city_root, &slot, &staging.join(name), register))
            .collect::<Result<Vec<_>, _>>()
    });
    let cleared = clear(&staging);
    let installed = shelved?;
    cleared?;
    Ok(installed)
}

fn stage(staging: &Path) -> Result<BTreeSet<&'static str>, AxError> {
    let names = SHIPPED
        .iter()
        .filter_map(|(path, _)| path.split_once('/').map(|(name, _)| name))
        .collect::<BTreeSet<&'static str>>();
    for (path, bytes) in SHIPPED {
        match path.split_once('/') {
            Some(_) => write(staging, path, bytes)?,
            None => {
                for name in &names {
                    write(staging, &format!("{name}/{path}"), bytes)?;
                }
            }
        }
    }
    Ok(names)
}

fn write(staging: &Path, path: &str, bytes: &[u8]) -> Result<(), AxError> {
    let target = path
        .split('/')
        .fold(staging.to_path_buf(), |at, segment| at.join(segment));
    match target.parent() {
        Some(parent) => std::fs::create_dir_all(parent),
        None => Ok(()),
    }
    .and_then(|()| std::fs::write(&target, bytes))
    .map_err(|err| staging_failed("stage a shipped skill", &target, &err))
}

fn clear(staging: &Path) -> Result<(), AxError> {
    match std::fs::remove_dir_all(staging) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(staging_failed(
            "clear the shipped skill staging",
            staging,
            &err,
        )),
    }
}

fn staging_failed(action: &str, path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action,
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "free space on this disk and make the city directory writable, then form the city again",
    )
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

    #[test]
    fn every_staged_skill_carries_the_licence_list() {
        let staged = tempfile::tempdir().unwrap();
        let names = stage(staged.path()).unwrap();
        let shared = SHIPPED.iter().find(|(path, _)| !path.contains('/'));
        assert!(shared.is_some(), "the table carries the licence list");
        let (_, licence) = shared.unwrap();
        assert!(!names.is_empty(), "the table carries a skill");
        for name in &names {
            let written = std::fs::read(staged.path().join(name).join("LICENSES.md")).unwrap();
            assert_eq!(written, *licence, "{name} carries the licence list it owes");
        }
    }
}
