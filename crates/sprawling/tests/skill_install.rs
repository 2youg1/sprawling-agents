// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The CAS binding of a skill install, end to end: `city::install_skill`
//! takes the store it registers with as a parameter, and this is the
//! production store answering it. The shelf index and every package file
//! resolve through verified reads of the manifest the install reported.

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
fn an_installed_skill_is_an_indexed_manifest_with_verified_bytes() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let document = stage.path().join("diagnosing.md");
    std::fs::write(&document, BODY).unwrap();

    let layout = kernel::layout::CityLayout::new(city_root.path());
    let mut cas = storage::Cas::open(&layout.cas()).unwrap();
    let slot = Slot::library("utilities").unwrap();
    let mut register = |bytes: &[u8]| cas.put(bytes).map_err(storage::StorageError::into_ax);

    let installed = install_skill(city_root.path(), &slot, &document, &mut register).unwrap();
    assert_eq!(installed.placed, Placed::Fresh);

    // A reinstall is idempotent on the shelf and in the store: the same
    // content is stored once for its lifetime.
    let again = install_skill(city_root.path(), &slot, &document, &mut register).unwrap();
    assert_eq!(again.placed, Placed::AlreadyShelved);
    assert_eq!(again.hash, installed.hash);

    let manifest = kernel::SkillManifest::decode(&cas.get(&installed.hash).unwrap()).unwrap();
    let document = manifest.document().unwrap();
    assert_eq!(document, &kernel::B3Hash::digest(BODY.as_bytes()));
    assert_eq!(cas.get(document).unwrap(), BODY.as_bytes());

    let library = city::Library::scan(city_root.path(), None, stage.path(), &|digest| {
        cas.get(digest).map_err(storage::StorageError::into_ax)
    })
    .unwrap();
    assert_eq!(library.all(), vec![&installed.holding]);
    assert!(
        !layout.library().join("utilities").exists(),
        "the shelf carries an index, not a second directory of skill bytes"
    );
}

/// Every package file has its own CAS object, and the manifest is their
/// canonical index rather than a second copy of the package's bytes.
#[test]
fn an_installed_package_is_a_manifest_of_per_file_cas_objects() {
    let city_root = tempfile::tempdir().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let package = stage.path().join("review");
    std::fs::create_dir_all(package.join("scripts")).unwrap();
    std::fs::write(package.join("SKILL.md"), BODY).unwrap();
    std::fs::write(package.join("scripts").join("run.sh"), "echo review\n").unwrap();

    let layout = kernel::layout::CityLayout::new(city_root.path());
    let mut cas = storage::Cas::open(&layout.cas()).unwrap();
    let slot = Slot::library("utilities").unwrap();
    let mut register = |bytes: &[u8]| cas.put(bytes).map_err(storage::StorageError::into_ax);

    let installed = install_skill(city_root.path(), &slot, &package, &mut register).unwrap();
    assert_eq!(installed.placed, Placed::Fresh);

    let manifest = kernel::SkillManifest::decode(&cas.get(&installed.hash).unwrap()).unwrap();
    for (path, bytes) in [
        ("SKILL.md", BODY.as_bytes()),
        ("scripts/run.sh", b"echo review\n".as_slice()),
    ] {
        let digest = manifest.file(path).unwrap();
        assert_eq!(digest, &kernel::B3Hash::digest(bytes));
        assert_eq!(cas.get(digest).unwrap(), bytes);
    }
    assert!(
        !layout.library().join("utilities/review").exists(),
        "no package directory remains beside the CAS"
    );
}
