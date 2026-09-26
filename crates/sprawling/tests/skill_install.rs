// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The CAS binding of a skill install, end to end: `city::install_skill`
//! takes the store it registers with as a parameter, and this is the
//! production store answering it. What lands on the shelf comes back
//! from the CAS under the hash the install reported, which is what makes
//! the shelved document's origin checkable after the fact.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use city::{Placed, Slot, install_skill};

const BODY: &str = "# Diagnose first\n\nbody\n";

#[test]
fn an_installed_skill_is_registered_in_the_cas_under_its_hash() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let document = stage.path().join("diagnosing.md");
    std::fs::write(&document, BODY).unwrap();

    let layout = kernel::layout::CityLayout::new(city_root.path());
    let mut cas = memory::Cas::open(&layout.cas()).unwrap();
    let slot = Slot::library("utilities").unwrap();
    let mut register = |bytes: &[u8]| cas.put(bytes).map_err(memory::MemoryError::into_ax);

    let installed = install_skill(city_root.path(), &slot, &document, &mut register).unwrap();
    assert_eq!(installed.placed, Placed::Fresh);

    // A reinstall is idempotent on the shelf and in the store: the same
    // content is stored once for its lifetime.
    let again = install_skill(city_root.path(), &slot, &document, &mut register).unwrap();
    assert_eq!(again.placed, Placed::AlreadyShelved);
    assert_eq!(again.hash, installed.hash);

    let from_shelf = std::fs::read(
        city_root
            .path()
            .join(kernel::RESERVED_PREFIX)
            .join(city::LIBRARY_DIR)
            .join("utilities")
            .join("diagnosing.md"),
    )
    .unwrap();
    let from_cas = cas.get(&installed.hash).unwrap();
    assert_eq!(
        from_shelf,
        BODY.as_bytes(),
        "the shelf holds the bytes that arrived"
    );
    assert_eq!(
        from_cas, from_shelf,
        "the CAS answers the same bytes under the hash the install reported"
    );
}

/// A package is registered as one blob, its canonical string, under the
/// hash the install reported; every item it carried is in that blob and
/// on the shelf under its own path.
#[test]
fn an_installed_package_is_registered_in_the_cas_as_one_blob() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let package = stage.path().join("review");
    std::fs::create_dir_all(package.join("scripts")).unwrap();
    std::fs::write(package.join("SKILL.md"), BODY).unwrap();
    std::fs::write(package.join("scripts").join("run.sh"), "echo review\n").unwrap();

    let layout = kernel::layout::CityLayout::new(city_root.path());
    let mut cas = memory::Cas::open(&layout.cas()).unwrap();
    let slot = Slot::library("utilities").unwrap();
    let mut register = |bytes: &[u8]| cas.put(bytes).map_err(memory::MemoryError::into_ax);

    let installed = install_skill(city_root.path(), &slot, &package, &mut register).unwrap();
    assert_eq!(installed.placed, Placed::Fresh);

    let shelved = city_root
        .path()
        .join(kernel::RESERVED_PREFIX)
        .join(city::LIBRARY_DIR)
        .join("utilities")
        .join("review");
    assert_eq!(
        std::fs::read(shelved.join("scripts").join("run.sh")).unwrap(),
        b"echo review\n",
        "every item lands under its own path"
    );
    let blob = cas.get(&installed.hash).unwrap();
    assert!(
        blob.starts_with(b"sprawling-skill-package/1\n"),
        "the blob is the package's canonical string"
    );
    for carried in [BODY.as_bytes(), b"echo review\n".as_slice()] {
        assert!(
            blob.windows(carried.len()).any(|window| window == carried),
            "every file the package carried is in the one blob"
        );
    }
}
