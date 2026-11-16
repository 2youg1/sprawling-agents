// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use kernel::{Address, B3Hash, EventRecord, RunId};

use super::holding::Views;

pub(super) type SkillPins = BTreeMap<(String, B3Hash), Vec<RunId>>;

impl Views {

    pub(super) fn fold_skill_pins(&mut self, record: &EventRecord) {
        let Ok(started) = record.data().read::<kernel::event::record::RunStarted>() else {
            return;
        };
        for pin in started.skills {
            let runs = self.skill_pins.entry((pin.name, pin.hash)).or_default();
            if !runs.contains(&record.run()) {
                runs.push(record.run());
            }
        }
    }
}

pub(super) fn skills_answer(
    city_root: &Path,
    building: &Address,
    pins: &SkillPins,
) -> Option<wire::SkillsAnswer> {
    let home = crate::home::Home::detect().ok()?;
    let store = kernel::layout::CityLayout::new(city_root).cas();
    let cas = storage::Cas::open(&store).ok()?;
    let read = |digest: &B3Hash| cas.get(digest).map_err(storage::StorageError::into_ax);
    let shelves = city::Library::scan(city_root, Some(building), home.path(), &read).ok()?;

    let rules = city::load(city_root, building).ok()?;
    let admitted: Vec<String> = rules.reading_room().to_vec();
    let skills = shelves
        .all()
        .into_iter()
        .map(|holding| wire::SkillLine {
            name: holding.name.clone(),
            section: holding.section.clone(),
            shelf: shelf_of(&holding.shelf),
            disclosure: holding.disclosure.clone(),
            hash: holding.hash,
            admitted: admitted.iter().any(|name| name == &holding.name),
            pinned_by: pinned_by(pins, &holding.name, &holding.hash),
        })
        .collect();
    Some(wire::SkillsAnswer {
        building: building.clone(),
        skills,
        missing: shelves.missing(&admitted),
        external_shelves: city::stated_skill_shelves(city_root).ok()?,
    })
}

fn pinned_by(pins: &SkillPins, name: &str, hash: &B3Hash) -> Vec<RunId> {
    pins.get(&(name.to_owned(), *hash))
        .cloned()
        .unwrap_or_default()
}

pub(super) fn shelf_of(shelf: &city::Shelf) -> wire::SkillShelf {
    match shelf {
        city::Shelf::Library => wire::SkillShelf::Library,
        city::Shelf::Building(addr) => wire::SkillShelf::Building(addr.clone()),
        city::Shelf::External { index, path } => wire::SkillShelf::External {
            index: *index,
            path: path.clone(),
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use std::path::Path;

    use super::*;

    fn shelved(city_root: &Path, slot: &city::Slot, name: &str, text: &str) {
        let source = city_root.join("fixtures").join(format!("{name}.md"));
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, text).unwrap();
        let store = kernel::layout::CityLayout::new(city_root).cas();
        let mut cas = storage::Cas::open(&store).unwrap();
        city::install_skill(city_root, slot, &source, &mut |bytes| {
            cas.put(bytes).map_err(storage::StorageError::into_ax)
        })
        .unwrap();
    }

    fn external(root: &Path, name: &str, text: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), text).unwrap();
    }

    #[test]
    fn every_shelf_reaches_the_page_with_the_place_it_sits_at() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let lab = Address::parse("lab").unwrap();
        city::create_building(dir.path(), &lab, city::BuildingTemplate::Minimal).unwrap();

        shelved(
            dir.path(),
            &city::Slot::library("utilities").unwrap(),
            "diffing",
            "The city's rule\n",
        );
        shelved(
            dir.path(),
            &city::Slot::building(&lab, "utilities").unwrap(),
            "diffing",
            "This lab's own rule\n",
        );
        shelved(
            dir.path(),
            &city::Slot::library("utilities").unwrap(),
            "kiln-firing",
            "# Firing\n\nbody\n",
        );
        let elsewhere = dir.path().join("elsewhere").join("skills");
        external(&elsewhere, "diagnosing-bugs", "# Diagnose first\n\nbody\n");
        std::fs::write(
            dir.path()
                .join(kernel::RESERVED_PREFIX)
                .join(city::CONFIG_FILE),
            format!(
                "[skills]\nshelves = [\"{}\"]\n",

                elsewhere.display().to_string().replace('\\', "/")
            ),
        )
        .unwrap();

        let mut views = Views::new(dir.path());
        let wire::Answer::Skills(answer) = views.answer(&wire::Query::Skills {
            building: lab.clone(),
        }) else {
            panic!("Skills answers with the shelves");
        };
        let named = |name: &str| {
            answer
                .skills
                .iter()
                .find(|row| row.name == name)
                .unwrap_or_else(|| panic!("{name} is on a shelf this building reads"))
        };

        match &named("diffing").shelf {
            wire::SkillShelf::Building(at) => assert!(at.is_within(&lab), "{at}"),
            wire::SkillShelf::Library | wire::SkillShelf::External { .. } => {
                panic!("the building's own copy is the nearer shelf")
            }
        }
        assert_eq!(named("diffing").disclosure, "This lab's own rule");
        match &named("kiln-firing").shelf {
            wire::SkillShelf::Library => {}
            wire::SkillShelf::Building(_) | wire::SkillShelf::External { .. } => {
                panic!("the city's stock is the library shelf")
            }
        }
        let outside = named("diagnosing-bugs");
        assert_eq!(
            outside.shelf,
            wire::SkillShelf::External {
                index: 0,
                path: "diagnosing-bugs/SKILL.md".to_owned(),
            },
            "an external row names its shelf and its path, and no address"
        );
        assert_eq!(outside.section, "", "that tree files no sections");
        assert_eq!(outside.disclosure, "Diagnose first");
    }

    #[test]
    fn rules_that_will_not_load_answer_unavailable_rather_than_admitting_nothing() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let lab = Address::parse("lab").unwrap();
        city::create_building(dir.path(), &lab, city::BuildingTemplate::Minimal).unwrap();
        std::fs::write(
            city::rules_path(dir.path(), &lab),
            "not a rules document [\n",
        )
        .unwrap();

        let mut views = Views::new(dir.path());
        let answer = views.answer(&wire::Query::Skills {
            building: lab.clone(),
        });
        let wire::Answer::Unavailable { query, .. } = answer else {
            panic!("rules that will not load are not a building that admits nothing");
        };
        assert_eq!(query, format!("Skills({})", lab.as_str()));
    }
}
