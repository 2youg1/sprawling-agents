// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A package installed whole: every item lands, the no-link rule
//! reaches every level, and a change to any item refuses the landing.

use super::*;

/// A package with more in it than its document: nested directories, an
/// empty one, and files beside `SKILL.md`.
fn whole_package(parent: &Path, name: &str) -> PathBuf {
    let dir = package(parent, name, BODY);
    std::fs::create_dir_all(dir.join("scripts/lib")).unwrap();
    std::fs::create_dir_all(dir.join("empty")).unwrap();
    std::fs::write(
        dir.join("scripts/run.sh"),
        "echo fired
",
    )
    .unwrap();
    std::fs::write(
        dir.join("scripts/lib/glaze.txt"),
        "cone 6
",
    )
    .unwrap();
    std::fs::write(
        dir.join("notes.md"),
        "loose end
",
    )
    .unwrap();
    dir
}

/// Every entry under `dir`, by relative path: a directory as `None`, a
/// file as its bytes.
fn tree(dir: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut open = vec![dir.to_path_buf()];
    while let Some(at) = open.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            let rel = path
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                open.push(path);
                out.push((rel, None));
            } else {
                out.push((rel, Some(std::fs::read(&path).unwrap())));
            }
        }
    }
    out.sort();
    out
}

/// A package lands whole: every directory and file under its own path,
/// registered as one blob, and read back by the scan as one holding.
#[test]
fn a_whole_package_lands_whole_under_its_name() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let src = whole_package(stage.path(), "review");
    let (seen, mut register) = counted();

    let installed = install(
        city_root.path(),
        &Slot::library("utilities").unwrap(),
        &src,
        &mut register,
    );
    assert!(
        installed.is_ok(),
        "a whole package is installed: {installed:?}"
    );
    let installed = installed.unwrap();

    assert_eq!(installed.placed, Placed::Fresh);
    assert_eq!(
        tree(&shelved_package(city_root.path(), "utilities", "review")),
        tree(&src),
        "every item landed under its own path, byte for byte"
    );
    assert_eq!(seen.borrow().len(), 1, "the package is registered once");
    assert_eq!(
        B3Hash::digest(&seen.borrow()[0]),
        installed.hash,
        "the one blob registered is the one the hash names"
    );
    let library = Library::scan(city_root.path(), None, Path::new("no-home")).unwrap();
    assert_eq!(library.all(), vec![&installed.holding]);
}

/// The no-link rule reaches every level of a package, and the refusal
/// names the item that broke it.
#[test]
fn a_link_deep_inside_a_package_is_refused_by_name() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let elsewhere = stage.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let src = whole_package(stage.path(), "review");
    let deep = src.join("scripts").join("lib").join("vault");
    make_link(&deep, &elsewhere);
    assert_is_link(&deep);

    let err =
        plan_install(city_root.path(), &Slot::library("utilities").unwrap(), &src).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(
        err.subject().contains("vault"),
        "the refusal names the linked item: {}",
        err.subject()
    );
}

/// An item deep inside a package that changes between the decision and
/// the landing refuses the whole install.
#[test]
fn an_item_deep_inside_changed_before_the_landing_refuses_the_whole() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let src = whole_package(stage.path(), "review");
    let slot = Slot::library("utilities").unwrap();
    let planned = plan_install(city_root.path(), &slot, &src);
    assert!(
        planned.is_ok(),
        "a whole package is planned: {:?}",
        planned.as_ref().err()
    );

    std::fs::write(
        src.join("scripts/lib/glaze.txt"),
        "cone 10
",
    )
    .unwrap();
    let (seen, mut register) = counted();
    let err = planned.unwrap().apply(&mut register).unwrap_err();
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(
        seen.borrow().is_empty(),
        "a refused install registers nothing"
    );
    assert!(!shelved_package(city_root.path(), "utilities", "review").exists());
}

/// `<name>.md` and `<name>/` side by side are two holdings under one
/// name: the scan would keep whichever it met last, so the shelf state
/// is refused rather than read as the package alone.
#[test]
fn a_name_held_both_as_a_document_and_as_a_package_is_refused() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let slot = Slot::library("utilities").unwrap();
    let src = package(stage.path(), "firing", BODY);
    let (_seen, mut register) = counted();
    install(city_root.path(), &slot, &src, &mut register).unwrap();
    std::fs::write(shelved(city_root.path(), "utilities", "firing"), BODY).unwrap();

    let err = plan_install(city_root.path(), &slot, &src).err();

    assert_eq!(
        err.as_ref().map(|err| *err.code()),
        Some(AxCode::InvalidArgs),
        "two holdings under one name refuse the install: {err:?}"
    );
}

/// A package past the size limit is refused by the lengths the handles
/// report, before its bytes are held.
#[test]
fn a_package_past_the_size_limit_is_refused() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let src = package(stage.path(), "kiln", BODY);
    std::fs::File::create(src.join("glaze.bin"))
        .unwrap()
        .set_len(super::super::precheck::walk::PACKAGE_BYTES_LIMIT)
        .unwrap();

    let err = plan_install(city_root.path(), &Slot::library("utilities").unwrap(), &src).err();

    assert_eq!(
        err.as_ref().map(|err| *err.code()),
        Some(AxCode::InvalidArgs),
        "a package past the limit is refused: {err:?}"
    );
}
