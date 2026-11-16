// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::layout::LIBRARY_DIR;

use super::super::test_store::Store;
use super::*;

/// A package on a section shelf is one holding: its manifest identifies
/// every file, while the catalog reads only SKILL.md's disclosure.
#[test]
fn a_package_on_a_section_shelf_is_one_holding_at_its_skill_file() {
    let dir = tempfile::tempdir().unwrap();
    let package = dir
        .path()
        .join(kernel::RESERVED_PREFIX)
        .join(LIBRARY_DIR)
        .join("utilities")
        .join("review");
    std::fs::create_dir_all(package.join("scripts")).unwrap();
    std::fs::write(package.join(SKILL_FILE), "# Review a diff\n\nbody\n").unwrap();
    std::fs::write(package.join("scripts").join("check.sh"), "echo\n").unwrap();

    let mut store = Store::new();
    let installed = store.migrate(dir.path(), None).into_iter().next().unwrap();
    let library = store.scan(dir.path(), None, Path::new("no-home"));

    assert_eq!(library.all(), vec![&installed.holding]);
    assert!(installed.previous.is_some());
    assert!(
        !package.exists(),
        "the index replaced the old package directory"
    );
}
