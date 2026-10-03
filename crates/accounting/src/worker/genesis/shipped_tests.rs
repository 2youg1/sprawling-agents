// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::*;

/// The skill names the source tree ships: every directory under the city
/// package's `skills/` that holds a `SKILL.md`, read off the tree rather
/// than listed, so a skill added there is a skill this test expects.
fn shipped_names() -> Vec<(String, String)> {
    let skills = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../city/skills");
    let mut names: Vec<(String, String)> = std::fs::read_dir(skills)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().join(kernel::layout::SKILL_FILE).is_file())
        .map(|entry| {
            (
                "shipped".to_owned(),
                entry.file_name().to_string_lossy().into_owned(),
            )
        })
        .collect();
    names.sort();
    names
}

fn shelved(city_root: &std::path::Path) -> Vec<(String, String)> {
    let home = city_root.join("no-home");
    let mut found: Vec<(String, String)> = city::Library::scan(city_root, None, &home)
        .unwrap()
        .all()
        .into_iter()
        .map(|holding| (holding.section.clone(), holding.name.clone()))
        .collect();
    found.sort();
    found
}

/// A new city has every built-in skill on its library shelf, under the
/// `shipped` section, and a second forming - refused, because history
/// starts once - leaves that shelf as it was (`crates/city/spec/Library/Install.lean`
/// §8-28c, city D20).
#[test]
fn a_formed_city_has_every_shipped_skill_on_its_shelf() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::genesis::form(dir.path(), Adopt::Nothing, crate::worker::fixture::hands())
        .unwrap();
    let expected = shipped_names();
    assert!(!expected.is_empty());
    assert_eq!(shelved(dir.path()), expected);

    crate::worker::genesis::form(dir.path(), Adopt::Nothing, crate::worker::fixture::hands())
        .unwrap_err();
    assert_eq!(shelved(dir.path()), expected);
}

/// Forming writes one `skill_shelved` per shipped skill, after line zero,
/// whose digest is the whole-package hash the shelf holds and whose
/// source is `Shipped` (kernel D23, `crates/city/spec/Library/Install.lean` §8-28c).
#[test]
fn a_formed_city_records_each_shipped_skill_it_shelved() {
    use kernel::event::record::{ShelvedFrom, SkillShelved};
    let dir = tempfile::tempdir().unwrap();
    let report =
        crate::worker::genesis::form(dir.path(), Adopt::Nothing, crate::worker::fixture::hands())
            .unwrap();
    let recorded: Vec<SkillShelved> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| kernel::EventRecord::parse_line(line).unwrap())
        .filter(|record| record.kind() == kernel::EventKind::SkillShelved)
        .map(|record| record.data().read::<SkillShelved>().unwrap())
        .collect();
    let slot = city::Slot::library(city::SHIPPED_SECTION).unwrap();
    let shelf = kernel::layout::CityLayout::new(dir.path())
        .library()
        .join(city::SHIPPED_SECTION);
    let expected: Vec<SkillShelved> = shipped_names()
        .into_iter()
        .map(|(_, name)| SkillShelved {
            digest: *city::plan_skill_install(dir.path(), &slot, &shelf.join(&name))
                .unwrap()
                .hash(),
            skill: name,
            source: ShelvedFrom::Shipped,
        })
        .collect();
    assert_eq!(recorded, expected);
}
