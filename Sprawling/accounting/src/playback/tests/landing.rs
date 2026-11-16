// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An export lands whole or not at all, never over another file, never
//! in protected metadata, and never on a name git tracks.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::Path;

use kernel::AxCode;
use kernel::layout::CityLayout;

use super::super::{Place, land};

fn listed(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn chosen(target: &Path) -> Result<(), AxCode> {
    land(Place::Chosen(target), b"{}").map_err(|err| *err.code())
}

#[test]
fn an_export_lands_whole_and_leaves_no_staged_copy() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("day.json");
    assert_eq!(
        (
            chosen(&target),
            std::fs::read(&target).ok(),
            listed(dir.path())
        ),
        (Ok(()), Some(b"{}".to_vec()), vec!["day.json".to_owned()])
    );
}

#[test]
fn an_export_never_overwrites_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("day.json");
    std::fs::write(&target, b"kept").unwrap();
    assert_eq!(
        (
            chosen(&target),
            std::fs::read(&target).unwrap(),
            listed(dir.path())
        ),
        (
            Err(AxCode::InvalidArgs),
            b"kept".to_vec(),
            vec!["day.json".to_owned()]
        )
    );
}

#[test]
fn an_export_never_lands_in_protected_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = dir.path().join(".sprawling");
    let git = dir.path().join(".GIT");
    std::fs::create_dir_all(&ledger).unwrap();
    std::fs::create_dir_all(&git).unwrap();
    assert_eq!(
        (
            chosen(&ledger.join("day.json")),
            chosen(&git.join("day.json")),
            listed(&ledger),
            listed(&git)
        ),
        (
            Err(AxCode::OutsideWriteDomain),
            Err(AxCode::OutsideWriteDomain),
            Vec::<String>::new(),
            Vec::<String>::new()
        )
    );
}

/// A repository at `root` whose index tracks `name`, which is then
/// deleted from the disk, and which ignores `ignored/`.
fn repository_tracking(root: &Path, name: &str) {
    let repo = git2::Repository::init(root).unwrap();
    std::fs::write(root.join(name), b"in history").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(name)).unwrap();
    index.write().unwrap();
    std::fs::remove_file(root.join(name)).unwrap();
    std::fs::write(root.join(".gitignore"), "/.sprawling/\n").unwrap();
}

#[test]
fn a_name_git_tracks_is_refused_though_it_is_gone_from_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    repository_tracking(dir.path(), "day.json");
    assert_eq!(
        (
            chosen(&dir.path().join("day.json")),
            dir.path().join("day.json").exists()
        ),
        (Err(AxCode::InvalidArgs), false)
    );
}

#[test]
fn an_export_into_the_city_lands_only_under_its_exports_and_out_of_history() {
    let dir = tempfile::tempdir().unwrap();
    repository_tracking(dir.path(), "kept.md");
    let exports = CityLayout::new(dir.path()).playback_exports().join("lab");
    let into = |file: &Path| {
        land(
            Place::Exports {
                city_root: dir.path(),
                file,
            },
            b"{}",
        )
        .map_err(|err| *err.code())
    };
    let landed = into(&exports.join("day.json"));
    let elsewhere = into(&dir.path().join("lab").join("day.json"));
    std::fs::remove_file(dir.path().join(".gitignore")).unwrap();
    let carried = into(&exports.join("carried.json"));
    assert_eq!(
        (landed, elsewhere, carried, listed(&exports)),
        (
            Ok(()),
            Err(AxCode::OutsideWriteDomain),
            Err(AxCode::OutsideWriteDomain),
            vec!["day.json".to_owned()]
        )
    );
}
