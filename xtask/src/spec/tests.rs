// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

/// The skeleton lands in the directory cargo metadata gives the lib.
#[test]
fn a_relocated_package_gets_its_spec_skeleton_in_its_own_directory() {
    let root = crate::root::fixture::relocated("spec");
    let made = super::run(&root, Some("k"));
    let landed = root.join("tools/k/k-SPEC.md").is_file();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(made.is_ok() && landed, "{made:?}");
}
