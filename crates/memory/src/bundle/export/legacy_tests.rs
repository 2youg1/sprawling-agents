// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Refusals of a v0.0.6 bundle, whose history travelled whole at
//! `city/.git` (memory-SPEC 8-12).

use super::super::fixture::city_with;
use super::tests::{committed_twice, copy_dir};
use super::*;

/// `open_bare` follows `objects/info/alternates` to object stores
/// elsewhere on this machine, so a forged bundle that names one would
/// pack objects the bundle never carried into the restored city.
#[test]
fn a_v006_bundle_that_borrows_objects_from_elsewhere_is_refused() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    committed_twice(home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    std::fs::remove_dir_all(carried.path().join(super::history::HISTORY)).unwrap();
    let whole = carried.path().join(CITY).join(".git");
    let copied = copy_dir(&home.path().join(".git"), &whole);
    let lender = tempfile::tempdir().unwrap();
    let lent = git2::Repository::init_bare(lender.path()).unwrap();
    let info = whole.join("objects").join("info");
    std::fs::create_dir_all(&info).unwrap();
    let objects = lent.path().join("objects");
    std::fs::write(
        info.join("alternates"),
        objects
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
    )
    .unwrap();
    let at = carried.path().join(MANIFEST);
    let mut manifest = Manifest::from_json(&std::fs::read(&at).unwrap(), &at).unwrap();
    manifest.files = manifest.files.saturating_add(copied).saturating_add(1);
    std::fs::write(&at, manifest.to_json()).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path()).map_err(|e| e.to_string());
    assert!(
        restored.as_ref().is_err_and(|e| e.contains("alternates")),
        "{restored:?}"
    );
}
