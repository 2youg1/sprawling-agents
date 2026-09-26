// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::layout::LIBRARY_DIR;

use super::super::Library;
use super::*;

/// A package is a directory on a section shelf: its `SKILL.md` is the
/// holding the catalog lists, and the files beside it stay on the shelf
/// for `read` to open by `<name>/<path>` rather than becoming holdings.
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

    let library = Library::scan(dir.path(), None, Path::new("no-home")).unwrap();

    let skill = format!(
        "{}/{LIBRARY_DIR}/utilities/review/{SKILL_FILE}",
        kernel::RESERVED_PREFIX
    );
    assert_eq!(
        library.all(),
        vec![&Holding::of(
            "review".to_owned(),
            "utilities".to_owned(),
            "# Review a diff\n\nbody\n",
            Shelf::Library(Address::parse(&skill).unwrap()),
        )]
    );
}
