// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Refusals of a bundle whose history is short or borrowed: a pack
//! gone missing, and a v0.0.6 repository at `city/.git` that reads
//! objects from elsewhere (memory-SPEC 8-12).

use super::super::fixture::city_with;
use super::tests::{committed_twice, copy_dir, log_of};
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
    manifest.history = super::history::Carried::default();
    std::fs::write(&at, manifest.to_json()).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path()).map_err(|e| e.to_string());
    assert!(
        restored.as_ref().is_err_and(|e| e.contains("alternates")),
        "{restored:?}"
    );
}

/// `open_bare` resolves `objects`, `refs` and `packed-refs` against the
/// path a `commondir` file names, so a forged bundle that names another
/// repository would pack that repository's history into the restored city.
#[test]
fn a_v006_bundle_that_borrows_a_common_dir_is_refused() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    committed_twice(home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    std::fs::remove_dir_all(carried.path().join(super::history::HISTORY)).unwrap();
    let whole = carried.path().join(CITY).join(".git");
    let copied = copy_dir(&home.path().join(".git"), &whole);
    let lender = tempfile::tempdir().unwrap();
    city_with(1, lender.path());
    committed_twice(lender.path());
    std::fs::write(
        whole.join("commondir"),
        lender
            .path()
            .join(".git")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
    )
    .unwrap();
    let at = carried.path().join(MANIFEST);
    let mut manifest = Manifest::from_json(&std::fs::read(&at).unwrap(), &at).unwrap();
    manifest.files = manifest.files.saturating_add(copied).saturating_add(1);
    manifest.history = super::history::Carried::default();
    std::fs::write(&at, manifest.to_json()).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path()).map_err(|e| e.to_string());
    assert!(
        restored.as_ref().is_err_and(|e| e.contains("commondir")),
        "{restored:?}"
    );
}

/// A bundle that lost its pack still has refs naming the objects
/// it held, so restoring it would land files and a ledger beside a
/// history that points at nothing.
#[test]
fn a_bundle_that_lost_its_pack_restores_nothing() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    committed_twice(home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    let history = carried.path().join(super::history::HISTORY);
    std::fs::remove_file(history.join("history.pack")).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path());
    assert!(restored.is_err(), "{restored:?}");
    assert_eq!(
        std::fs::read_dir(elsewhere.path()).unwrap().count(),
        0,
        "a refused restore leaves the city root empty"
    );
}

/// A v0.0.6 export copied the repository whole into `city/.git` and
/// counted its files in the manifest; its objects and refs come back,
/// its hooks and config do not.
#[test]
fn a_v006_bundle_brings_back_its_history_and_not_its_hooks() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    let before = committed_twice(home.path());
    let hooks = home.path().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(hooks.join("post-checkout"), b"escalate").unwrap();
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    std::fs::remove_dir_all(carried.path().join(super::history::HISTORY)).unwrap();
    let whole = copy_dir(
        &home.path().join(".git"),
        &carried.path().join(CITY).join(".git"),
    );
    let at = carried.path().join(MANIFEST);
    let mut manifest = Manifest::from_json(&std::fs::read(&at).unwrap(), &at).unwrap();
    manifest.files = manifest.files.saturating_add(whole);
    manifest.history = super::history::Carried::default();
    std::fs::write(&at, manifest.to_json()).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path()).map_err(|e| e.to_string());
    assert_eq!(restored.map(|_| ()), Ok(()));
    assert_eq!(log_of(elsewhere.path()), before);
    let git = elsewhere.path().join(".git");
    assert!(!git.join("hooks").join("post-checkout").exists());
}
