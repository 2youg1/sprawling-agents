// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

use std::cell::RefCell;
use std::rc::Rc;

use kernel::layout::LIBRARY_DIR;
use kernel::{AxCode, B3Hash};

use crate::library::Library;

mod whole;

const BODY: &str = "# Firing a kiln\n\nbody\n";

/// One skill package in the layout a shelf outside the city files them:
/// a directory named after the skill, holding `SKILL.md`.
fn package(parent: &Path, name: &str, text: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), text).unwrap();
    dir
}

/// The file a shelved skill lands as.
fn shelved(city_root: &Path, section: &str, name: &str) -> PathBuf {
    city_root
        .join(kernel::RESERVED_PREFIX)
        .join(LIBRARY_DIR)
        .join(section)
        .join(format!("{name}.md"))
}

/// Where a whole package lands: a directory named after the skill.
fn shelved_package(city_root: &Path, section: &str, name: &str) -> PathBuf {
    city_root
        .join(kernel::RESERVED_PREFIX)
        .join(LIBRARY_DIR)
        .join(section)
        .join(name)
}

/// A registrar that records every byte string it was handed and answers
/// the content hash, standing in for the assembly layer's CAS binding.
fn counted() -> (
    Rc<RefCell<Vec<Vec<u8>>>>,
    impl FnMut(&[u8]) -> Result<B3Hash, AxError>,
) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mine = Rc::clone(&seen);
    let register = move |bytes: &[u8]| {
        mine.borrow_mut().push(bytes.to_vec());
        Ok(B3Hash::digest(bytes))
    };
    (seen, register)
}

/// Places `target` at `link` as a link, on a machine that has no symlink
/// privilege: Windows gets a junction, which `symlink_metadata` reports
/// through the same name-surrogate predicate a symlink is reported by.
#[cfg(windows)]
fn make_link(link: &Path, target: &Path) {
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .unwrap();
    assert!(made.success(), "the fixture could not make a link");
}

#[cfg(unix)]
fn make_link(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

/// The fixture is worthless unless what it built is a link, and that is
/// an assertion about this machine rather than about the code under test.
fn assert_is_link(path: &Path) {
    let meta = std::fs::symlink_metadata(path).unwrap();
    assert!(meta.file_type().is_symlink(), "{path:?} is not a link");
}

#[test]
fn an_installed_skill_is_shelved_and_read_back_whole() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let src = package(stage.path(), "firing", BODY);
    let (seen, mut register) = counted();

    let installed = install(
        city_root.path(),
        &Slot::library("utilities").unwrap(),
        &src,
        &mut register,
    )
    .unwrap();

    assert_eq!(installed.placed, Placed::Fresh);
    assert_eq!(
        std::fs::read(shelved_package(city_root.path(), "utilities", "firing").join("SKILL.md"))
            .unwrap(),
        BODY.as_bytes(),
        "what landed is what arrived, byte for byte"
    );
    assert_eq!(seen.borrow().len(), 1, "the bytes were registered once");
    assert_eq!(B3Hash::digest(&seen.borrow()[0]), installed.hash);
    // The scan reads back exactly the holding install reported: one name
    // is one holding, and the two derivations cannot disagree.
    let library = Library::scan(city_root.path(), None, Path::new("no-home")).unwrap();
    assert_eq!(library.all(), vec![&installed.holding]);
}

/// A resident's own document arrives as a file rather than a package,
/// and lands the same way.
#[test]
fn a_residents_own_document_is_shelved_the_same_way() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let document = stage.path().join("hand-written.md");
    std::fs::write(&document, "# Hand written\n").unwrap();
    let (seen, mut register) = counted();

    let installed = install(
        city_root.path(),
        &Slot::library("notes").unwrap(),
        &document,
        &mut register,
    )
    .unwrap();

    assert_eq!(installed.holding.name, "hand-written");
    assert_eq!(
        std::fs::read(shelved(city_root.path(), "notes", "hand-written")).unwrap(),
        b"# Hand written\n"
    );
    assert_eq!(seen.borrow().len(), 1);
}

#[test]
fn reinstalling_the_same_bytes_is_idempotent() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let src = package(stage.path(), "firing", BODY);
    let slot = Slot::library("utilities").unwrap();
    let (_seen, mut register) = counted();

    let first = install(city_root.path(), &slot, &src, &mut register).unwrap();
    let second = install(city_root.path(), &slot, &src, &mut register).unwrap();

    assert_eq!(first.placed, Placed::Fresh);
    assert_eq!(second.placed, Placed::AlreadyShelved);
    assert_eq!(second.holding, first.holding, "one name holds one holding");
    assert_eq!(
        std::fs::read(shelved_package(city_root.path(), "utilities", "firing").join("SKILL.md"))
            .unwrap(),
        BODY.as_bytes(),
        "a reinstall writes no byte"
    );
}

/// The recheck is the whole point of the gap between deciding and
/// landing: a package that moved underneath is refused entire, and
/// neither the shelves nor the store are touched.
#[test]
fn a_package_changed_while_it_was_being_installed_is_refused_whole() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let slot = Slot::library("utilities").unwrap();

    // The document is swapped after it was checked.
    let swapped = package(stage.path(), "swapped", BODY);
    let planned = plan_install(city_root.path(), &slot, &swapped).unwrap();
    std::fs::write(swapped.join("SKILL.md"), "# Something else\n").unwrap();
    let (seen, mut register) = counted();
    let err = planned.apply(&mut register).unwrap_err();
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(
        seen.borrow().is_empty(),
        "a refused install registers nothing"
    );
    assert!(
        !shelved_package(city_root.path(), "utilities", "swapped").exists(),
        "a refused install leaves the shelves untouched"
    );

    // The directory itself is swapped after it was checked: another
    // entry appears, and what was checked is no longer what is there.
    let grown = package(stage.path(), "grown", BODY);
    let planned = plan_install(city_root.path(), &slot, &grown).unwrap();
    std::fs::write(grown.join("notes.md"), "loose end\n").unwrap();
    let err = planned.apply(&mut register).unwrap_err();
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(seen.borrow().is_empty());
    assert!(!shelved_package(city_root.path(), "utilities", "grown").exists());
}

/// A package reached through a link is refused whatever the link points
/// at: its bytes are not the package's to file, and where it points can
/// change between the check and the landing.
#[test]
fn a_package_reached_through_a_link_is_always_refused() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let slot = Slot::library("utilities").unwrap();

    // The package directory itself is a link.
    let real = package(stage.path(), "real", BODY);
    let linked = stage.path().join("linked");
    make_link(&linked, &real);
    assert_is_link(&linked);
    let err = plan_install(city_root.path(), &slot, &linked).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);

    // The document inside the package is a link.
    let holder = stage.path().join("holder");
    std::fs::create_dir_all(&holder).unwrap();
    let pkg = stage.path().join("inner");
    std::fs::create_dir_all(&pkg).unwrap();
    let inner = pkg.join("SKILL.md");
    make_link(&inner, &holder);
    assert_is_link(&inner);
    let err = plan_install(city_root.path(), &slot, &pkg).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(
        err.subject().contains("SKILL.md"),
        "a refusal names the linked item: {}",
        err.subject()
    );
}

/// One name holds one holding, so a name taken on the shelf refuses a
/// different skill under it - and refuses it in another section too,
/// where the scan's by-name key would let the two silently shadow.
#[test]
fn a_taken_name_refuses_a_different_skill() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let slot = Slot::library("utilities").unwrap();
    let (seen, mut register) = counted();

    let src = package(stage.path(), "firing", BODY);
    install(city_root.path(), &slot, &src, &mut register).unwrap();

    // Same name, different bytes: the shelf keeps what it had.
    std::fs::write(src.join("SKILL.md"), "# Another firing rule\n").unwrap();
    let err = plan_install(city_root.path(), &slot, &src).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert_eq!(
        std::fs::read(shelved_package(city_root.path(), "utilities", "firing").join("SKILL.md"))
            .unwrap(),
        BODY.as_bytes(),
        "the shelved skill is the one that was there"
    );

    // Same name under another section of the same shelf.
    let err = plan_install(city_root.path(), &Slot::library("domain").unwrap(), &src).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert_eq!(
        seen.borrow().len(),
        1,
        "a refused install registers nothing"
    );
}

/// A refusal about a name that is already on the shelf names the
/// section that holds it: the person has to know which filing to take
/// off the shelf, and the section being installed into is not it.
#[test]
fn a_refusal_names_the_section_that_holds_the_taken_name() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let src = package(stage.path(), "firing", BODY);
    let (_seen, mut register) = counted();

    install(
        city_root.path(),
        &Slot::library("utilities").unwrap(),
        &src,
        &mut register,
    )
    .unwrap();

    let err = plan_install(city_root.path(), &Slot::library("domain").unwrap(), &src).unwrap_err();
    assert!(
        err.subject().contains("utilities"),
        "the refusal names the section holding `firing`, not the one asked for: {}",
        err.subject()
    );
}
