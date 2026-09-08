// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::policy::{self, ModelPool};

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

#[test]
fn a_created_building_is_read_back_by_the_citys_own_parser() {
    let dir = tempfile::tempdir().unwrap();
    let building = create(dir.path(), &addr("lab"), BuildingTemplate::Minimal).unwrap();
    assert_eq!(building.addr(), &addr("lab"));

    let rules = policy::load(dir.path(), &addr("lab")).unwrap();
    assert!(!rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::Any);
    // With nothing declared, a new building may write itself and no more.
    assert_eq!(rules.write_domain().unwrap().prefixes().count(), 1);

    let text =
        std::fs::read_to_string(crate::policy::building_path(dir.path(), &addr("lab"))).unwrap();
    assert!(
        text.contains("lab"),
        "a building's own rules name the building"
    );
    assert!(!text.contains(NAME_PLACEHOLDER));
    assert!(
        building
            .root(dir.path())
            .join(crate::spine_files::ROADMAP_FILE)
            .exists(),
        "a building exists with its plan, not only with its rules"
    );
}

#[test]
fn the_confidential_template_produces_a_building_the_city_treats_as_confidential() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), &addr("vault"), BuildingTemplate::Confidential).unwrap();

    let rules = policy::load(dir.path(), &addr("vault")).unwrap();
    assert!(rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::LocalOnly);
    assert!(rules.egress().is_empty());
}

#[test]
fn a_second_birth_is_refused_and_the_first_rules_survive() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), &addr("vault"), BuildingTemplate::Confidential).unwrap();

    let err = create(dir.path(), &addr("vault"), BuildingTemplate::Minimal).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("already has rules"));
    assert!(
        policy::load(dir.path(), &addr("vault"))
            .unwrap()
            .policy()
            .confidential,
        "the refusal left the confidential rules in place"
    );
}

#[test]
fn a_room_address_is_refused_and_the_refusal_names_the_building_to_create() {
    let dir = tempfile::tempdir().unwrap();
    let err = create(dir.path(), &addr("lab/room1"), BuildingTemplate::Minimal).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("`lab`"));
    assert!(!dir.path().join("lab").join("room1").exists());
    assert!(
        !dir.path().join("lab").exists(),
        "a refused building leaves nothing behind"
    );
}

#[test]
fn the_reserved_subtree_belongs_to_no_building() {
    let dir = tempfile::tempdir().unwrap();
    let err = Building::of(&addr(".sprawling/cas/ab")).unwrap_err();
    assert!(err.recovery().contains("reserved subtree"));

    let err = create(dir.path(), &addr(".sprawling"), BuildingTemplate::Minimal).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(!dir.path().join(".sprawling").join(BUILDING_FILE).exists());
}

#[test]
fn a_building_holds_its_own_rooms_and_no_others() {
    let lab = Building::of(&addr("lab/room1")).unwrap();
    assert_eq!(lab.addr(), &addr("lab"));
    assert!(lab.holds(&addr("lab")));
    assert!(lab.holds(&addr("lab/room1/notes.md")));
    assert!(!lab.holds(&addr("laboratory/room1")));
    assert!(!lab.holds(&addr("vault")));
}

#[test]
fn an_unknown_template_is_refused_with_the_set_this_version_lays_out() {
    let err = BuildingTemplate::parse("workshop").unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("minimal"));
    assert!(err.recovery().contains("confidential"));
    assert_eq!(
        BuildingTemplate::parse(BuildingTemplate::Confidential.name()).unwrap(),
        BuildingTemplate::Confidential
    );
}

#[test]
fn the_record_carries_the_address_and_the_template_and_nothing_else() {
    let building = Building::of(&addr("lab")).unwrap();
    let payload = created_payload(&building, BuildingTemplate::Confidential).unwrap();
    let map = payload.as_map();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("addr").and_then(|v| v.as_str()), Some("lab"));
    assert_eq!(
        map.get("template").and_then(|v| v.as_str()),
        Some("confidential")
    );
}

#[test]
fn adopting_an_existing_directory_keeps_every_file_it_found() {
    let city = tempfile::tempdir().unwrap();
    let repo = city.path().join("imported");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(repo.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(repo.join("Roadmap.md"), "# my own roadmap\n").unwrap();

    let building = adopt(city.path(), &addr("imported")).unwrap();
    assert_eq!(building.addr().as_str(), "imported");
    // What was there is untouched; what was missing is laid.
    assert_eq!(
        std::fs::read_to_string(repo.join("src").join("main.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("Roadmap.md")).unwrap(),
        "# my own roadmap\n",
        "an adopted roadmap is the owner's, not the template's"
    );
    // The rules land in the building's reserved subtree; the spine
    // files a person reads and writes stay where they were.
    assert!(
        crate::policy::building_path(city.path(), &addr("imported")).is_file(),
        "an adopted directory has no rules of its own"
    );
    assert!(!repo.join("BUILDING.md").exists());
    assert!(repo.join("Memo.md").is_file());
    // Adopting twice refuses: it is already a building.
    assert!(adopt(city.path(), &addr("imported")).is_err());
    // Adopting nothing refuses toward create.
    let err = adopt(city.path(), &addr("ghost")).unwrap_err();
    assert!(err.recovery().contains("create"), "{err}");
    // The record says it was an adoption.
    let payload = adopted_payload(&building).unwrap();
    assert_eq!(
        payload.as_map().get("adopted"),
        Some(&serde_json::Value::Bool(true))
    );
}
