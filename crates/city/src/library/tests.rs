// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::layout::{BUILDING_SHELF, LIBRARY_DIR};

use super::*;

fn stocked(root: &Path) {
    let shelves = root.join(kernel::RESERVED_PREFIX).join(LIBRARY_DIR);
    for (section, name, text) in [
        (
            "utilities",
            "unit-tests",
            "# Writing a test that earns its place\n\nbody",
        ),
        ("utilities", "diffing", "How to read a diff\n"),
        ("domain", "kiln-firing", "# Firing schedules\n"),
    ] {
        let dir = shelves.join(section);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.md")), text).unwrap();
    }
}

#[test]
fn a_city_with_no_library_has_an_empty_one_rather_than_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::scan(dir.path(), None).unwrap();
    assert!(library.all().is_empty());
    assert!(library.reading_room(&["anything".to_owned()]).is_empty());
}

#[test]
fn a_building_takes_only_what_its_list_admits() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let library = Library::scan(dir.path(), None).unwrap();
    assert_eq!(library.all().len(), 3, "the shelves hold everything");
    let admitted = library.reading_room(&["unit-tests".to_owned(), "diffing".to_owned()]);
    let names: Vec<&str> = admitted.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["diffing", "unit-tests"],
        "a thousand may sit on the shelf; this building reads two"
    );
}

#[test]
fn the_one_line_is_the_authors_line() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let library = Library::scan(dir.path(), None).unwrap();
    let holding = library
        .all()
        .into_iter()
        .find(|h| h.name == "unit-tests")
        .unwrap();
    assert_eq!(holding.disclosure, "Writing a test that earns its place");
    assert_eq!(holding.section, "utilities");
}

#[test]
fn a_name_on_the_list_that_is_not_on_the_shelf_is_reported_to_whoever_wrote_it() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let library = Library::scan(dir.path(), None).unwrap();
    let asked = vec!["diffing".to_owned(), "imagined".to_owned()];
    assert_eq!(library.reading_room(&asked).len(), 1);
    assert_eq!(library.missing(&asked), vec!["imagined".to_owned()]);
}

#[test]
fn a_holding_lives_where_no_run_may_write() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let library = Library::scan(dir.path(), None).unwrap();
    let holding = library.all()[0];
    assert!(
        holding.addr.is_reserved(),
        "a resident may read the stock and may not restock it"
    );
}

/// A building keeps the skills only it uses on its own shelf, inside
/// its own directory, and still cannot restock them.
#[test]
fn a_building_keeps_its_own_shelf_and_the_nearer_shelf_wins() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let lab = Address::parse("lab").unwrap();
    let own = dir
        .path()
        .join("lab")
        .join(kernel::RESERVED_PREFIX)
        .join(BUILDING_SHELF)
        .join("utilities");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(own.join("kiln.md"), "Firing a kiln in this lab\n").unwrap();
    std::fs::write(own.join("unit-tests.md"), "This lab's own rule for tests\n").unwrap();

    let library = Library::scan(dir.path(), Some(&lab)).unwrap();
    let names: Vec<&str> = library.all().iter().map(|h| h.name.as_str()).collect();
    assert!(names.contains(&"kiln"), "{names:?}");
    assert!(names.contains(&"diffing"), "the city's shelf is still read");

    let mine = library
        .all()
        .into_iter()
        .find(|h| h.name == "unit-tests")
        .unwrap();
    assert_eq!(
        mine.disclosure, "This lab's own rule for tests",
        "the nearer shelf wins, as the configuration ladder already does"
    );
    assert!(
        mine.addr.is_reserved(),
        "a building's own shelf is still outside its write domain: {}",
        mine.addr.as_str()
    );

    // Another building sees only the city's shelf.
    let other = Library::scan(dir.path(), Some(&Address::parse("vault").unwrap())).unwrap();
    assert!(!other.all().iter().any(|h| h.name == "kiln"));
}

/// A reading room admits by name, so the shelves file by name: a
/// building's copy of a skill replaces the city's even when the
/// two were filed under different sections. Filed by section and
/// name, both copies survived and one name had two answers.
#[test]
fn a_nearer_copy_filed_under_another_section_still_replaces_the_citys() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let lab = Address::parse("lab").unwrap();
    let own = dir
        .path()
        .join("lab")
        .join(kernel::RESERVED_PREFIX)
        .join(BUILDING_SHELF)
        .join("domain");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(
        own.join("unit-tests.md"),
        "This lab files its test rule under domain\n",
    )
    .unwrap();

    let library = Library::scan(dir.path(), Some(&lab)).unwrap();
    let admitted = library.reading_room(&["unit-tests".to_owned()]);
    assert_eq!(
        admitted.len(),
        1,
        "one name admits one holding, whatever section it sits in"
    );
    let held = admitted.first().unwrap();
    assert_eq!(held.disclosure, "This lab files its test rule under domain");
    assert_eq!(held.section, "domain");
    assert_eq!(
        library.all().len(),
        3,
        "the replaced copy left the shelves rather than sitting beside its replacement"
    );
}
