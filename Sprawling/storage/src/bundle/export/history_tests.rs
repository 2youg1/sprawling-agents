// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::fixture::city_with;
use super::tests::committed_twice;
use super::*;

#[test]
fn restore_refuses_embedded_repository_metadata_before_writing_files() {
    for file in [
        "objects/info/alternates",
        "commondir",
        "hooks/post-checkout",
    ] {
        let home = tempfile::tempdir().unwrap();
        city_with(1, home.path());
        let bundle = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), bundle.path()).unwrap();
        let file = bundle.path().join(CITY).join(".git").join(file);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, b"foreign metadata").unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(Bundle::restore(bundle.path(), elsewhere.path()).is_err());
        assert_eq!(std::fs::read_dir(elsewhere.path()).unwrap().count(), 0);
    }
}

#[test]
fn a_bundle_that_lost_its_pack_restores_nothing() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    committed_twice(home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    let history = carried.path().join(super::history::HISTORY).join("lab");
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
