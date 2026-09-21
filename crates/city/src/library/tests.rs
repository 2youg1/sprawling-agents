// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::layout::{BUILDING_SHELF, LIBRARY_DIR};

use super::*;

/// A scan of a city whose own configuration names no external shelf, so
/// the home directory the scan takes is never joined to anything.
fn scan(city_root: &Path, building: Option<&Address>) -> Result<Library, AxError> {
    Library::scan(city_root, building, Path::new("no-home"))
}

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
    let library = scan(dir.path(), None).unwrap();
    assert!(library.all().is_empty());
    assert!(library.reading_room(&["anything".to_owned()]).is_empty());
}

#[test]
fn a_building_takes_only_what_its_list_admits() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let library = scan(dir.path(), None).unwrap();
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
    let library = scan(dir.path(), None).unwrap();
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
    let library = scan(dir.path(), None).unwrap();
    let asked = vec!["diffing".to_owned(), "imagined".to_owned()];
    assert_eq!(library.reading_room(&asked).len(), 1);
    assert_eq!(library.missing(&asked), vec!["imagined".to_owned()]);
}

#[test]
fn a_holding_lives_where_no_run_may_write() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let library = scan(dir.path(), None).unwrap();
    let holding = library.all()[0];
    let at = holding
        .shelf
        .address()
        .expect("a city shelf is addressable");
    assert!(
        at.is_reserved(),
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

    let library = scan(dir.path(), Some(&lab)).unwrap();
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
        mine.shelf
            .address()
            .expect("a building shelf is addressable")
            .is_reserved(),
        "a building's own shelf is still outside its write domain: {:?}",
        mine.shelf
    );

    // Another building sees only the city's shelf.
    let other = scan(dir.path(), Some(&Address::parse("vault").unwrap())).unwrap();
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

    let library = scan(dir.path(), Some(&lab)).unwrap();
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

/// Writes the city's own configuration, which is where a shelf outside
/// the city is named.
fn configured(root: &Path, shelves: &[&str]) {
    let governed = root.join(kernel::RESERVED_PREFIX);
    std::fs::create_dir_all(&governed).unwrap();
    let listed: Vec<String> = shelves.iter().map(|one| format!("\"{one}\"")).collect();
    std::fs::write(
        governed.join(kernel::layout::CONFIG_FILE),
        format!("[skills]\nshelves = [{}]\n", listed.join(", ")),
    )
    .unwrap();
}

/// One skill in the layout a shelf outside the city uses: a directory
/// named after the skill, holding `SKILL.md`.
fn external_skill(shelf: &Path, name: &str, text: &str) {
    let dir = shelf.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), text).unwrap();
}

/// A shelf outside the city is mounted read-only: its skills are on the
/// shelves, and each says which shelf and which path, because it has no
/// address in the city to be opened by.
#[test]
fn an_external_shelf_is_mounted_and_its_holdings_have_no_address() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let shelf = home.join("agents").join("skills");
    external_skill(&shelf, "diagnosing-bugs", "# Diagnose first\n\nbody\n");
    configured(dir.path(), &["~/agents/skills"]);

    let library = Library::scan(dir.path(), None, &home).unwrap();
    let held = library
        .all()
        .into_iter()
        .find(|h| h.name == "diagnosing-bugs")
        .unwrap();
    assert_eq!(held.disclosure, "Diagnose first");
    assert_eq!(
        held.shelf,
        Shelf::External {
            index: 0,
            path: "diagnosing-bugs/SKILL.md".to_owned(),
        }
    );
    assert!(
        held.shelf.address().is_none(),
        "a file outside the city has no address, and a made-up one would send a reader to \
         a file that is not there"
    );
    assert!(
        library.reading_room(&["diagnosing-bugs".to_owned()]).len() == 1,
        "a building may admit it like any other skill"
    );
}

/// The nearest shelf wins by name, and the city's own stock is nearer
/// than a directory belonging to another program: a name this city
/// keeps means the city's skill.
#[test]
fn the_citys_own_stock_beats_an_external_shelf_of_the_same_name() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let home = dir.path().join("home");
    let shelf = home.join("agents").join("skills");
    external_skill(&shelf, "unit-tests", "Another program's rule for tests\n");
    configured(dir.path(), &["~/agents/skills"]);

    let library = Library::scan(dir.path(), None, &home).unwrap();
    assert_eq!(
        library.all().len(),
        3,
        "the city's three filed, and the external copy of one of them lost"
    );
    let held = library
        .all()
        .into_iter()
        .find(|h| h.name == "unit-tests")
        .unwrap();
    assert_eq!(held.disclosure, "Writing a test that earns its place");
    assert!(held.shelf.address().is_some(), "the city's own copy won");
}

/// The catalog groups by shelf before it looks at a section, so the
/// city's own stock comes first even when an external holding has no
/// section to sort under and the building's own is filed late. Sorting
/// every holding by section once put the external rows on top.
#[test]
fn the_catalog_lists_the_citys_stock_before_a_buildings_own_and_an_external_shelf() {
    let dir = tempfile::tempdir().unwrap();
    stocked(dir.path());
    let home = dir.path().join("home");
    external_skill(&home.join("tools"), "aa-external", "From elsewhere\n");
    configured(dir.path(), &["~/tools"]);
    let lab = Address::parse("lab").unwrap();
    let own = dir
        .path()
        .join("lab")
        .join(kernel::RESERVED_PREFIX)
        .join(BUILDING_SHELF)
        .join("zz-section");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(own.join("zz-building.md"), "The building's own\n").unwrap();

    let library = Library::scan(dir.path(), Some(&lab), &home).unwrap();
    let named: Vec<&str> = library
        .all()
        .into_iter()
        .map(|held| held.name.as_str())
        .collect();
    assert_eq!(
        named,
        vec![
            "kiln-firing",
            "diffing",
            "unit-tests",
            "zz-building",
            "aa-external",
        ],
        "city stock, then the building's own, then the external shelf"
    );
}

/// Two shelves are read in the order the configuration lists them, and
/// each holding says which one it came from.
#[test]
fn the_second_shelf_is_the_second_index() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    external_skill(&home.join("one"), "first", "From the first shelf\n");
    external_skill(&home.join("two"), "second", "From the second shelf\n");
    configured(dir.path(), &["~/one", "~/two"]);

    let library = Library::scan(dir.path(), None, &home).unwrap();
    let named: Vec<(&str, Shelf)> = library
        .all()
        .into_iter()
        .map(|held| (held.name.as_str(), held.shelf.clone()))
        .collect();
    assert_eq!(
        named,
        vec![
            (
                "first",
                Shelf::External {
                    index: 0,
                    path: "first/SKILL.md".to_owned(),
                }
            ),
            (
                "second",
                Shelf::External {
                    index: 1,
                    path: "second/SKILL.md".to_owned(),
                }
            ),
        ]
    );
}

/// A shelf that is not there is an empty shelf, and a shelf that is a
/// file is a person's mistake rather than a run's silence.
#[test]
fn a_missing_external_shelf_is_empty_and_a_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    configured(dir.path(), &["~/not-here"]);
    assert!(
        Library::scan(dir.path(), None, &home)
            .unwrap()
            .all()
            .is_empty()
    );

    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("not-here"), "a file, not a shelf\n").unwrap();
    let err = Library::scan(dir.path(), None, &home).unwrap_err();
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
    assert!(err.subject().contains("not-here"), "{}", err.subject());
}

/// A path in the configuration that this machine cannot open as written
/// is refused where it was written, naming it.
#[test]
fn a_relative_shelf_path_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    configured(dir.path(), &["../elsewhere"]);
    let err = Library::scan(dir.path(), None, dir.path()).unwrap_err();
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
    assert!(
        err.recovery().contains("absolute"),
        "a refusal that will not say the rule cannot be acted on: {}",
        err.recovery()
    );
}
