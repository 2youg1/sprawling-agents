// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one building can do, joined to which of its runs used it.
//!
//! **The shelves are read at the moment of asking.** `city::Library`
//! scans them, and an index kept beside it would be a second copy of
//! what the disk says - the same rule `BuildingView` follows for a
//! building's own documents.
//!
//! **The pins come from the history.** Every `run_started` records the
//! skills that run's reading room admitted and what each hashed to, so
//! "which runs used this" is a fold rather than a second scan, and a
//! skill edited since is visible as a run pinned to a hash no shelf
//! holds any more.

use kernel::{Address, B3Hash, EventRecord, RunId};

use super::holding::Views;

impl Views {
    /// Every shelf one building reads from, with the runs that pinned
    /// each holding.
    ///
    /// `None` for a building whose shelves will not scan, which is a
    /// broken installation rather than an empty one: `Unavailable` says
    /// the view could not look, and an empty list would say this
    /// building can do nothing.
    pub(super) fn skills_answer(&self, building: &Address) -> Option<channels::SkillsAnswer> {
        let home = crate::home::Home::detect().ok()?;
        let shelves = city::Library::scan(&self.city_root, Some(building), home.path()).ok()?;
        // What this building's reading room admits, so a page can show
        // the stock and the choice in one list. A building with no
        // rules file admits nothing, which is what a fresh building is.
        let rules = city::load(&self.city_root, building).ok();
        let admitted: Vec<String> = rules
            .as_ref()
            .map(|held| held.reading_room().to_vec())
            .unwrap_or_default();
        let skills = shelves
            .all()
            .into_iter()
            .map(|holding| channels::SkillLine {
                name: holding.name.clone(),
                section: holding.section.clone(),
                shelf: shelf_of(&holding.shelf),
                disclosure: holding.disclosure.clone(),
                hash: holding.hash,
                admitted: admitted.iter().any(|name| name == &holding.name),
                pinned_by: self.pinned_by(&holding.name, &holding.hash),
            })
            .collect();
        Some(channels::SkillsAnswer {
            building: building.clone(),
            skills,
            missing: shelves.missing(&admitted),
        })
    }

    /// Files which skills one `run_started` says that run was frozen
    /// with.
    ///
    /// What a page answers with this is "which runs read these exact
    /// bytes", so a hash is required of every pin: a record holding a
    /// pin with no hash is left out whole rather than filed under a
    /// guess, and `kernel::event::record::SkillPin` is where that
    /// requirement is stated.
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

    /// The runs frozen with this exact name and hash pinned, oldest
    /// first.
    ///
    /// The hash as well as the name, because a skill edited between two
    /// runs is two different documents under one name, and a list that
    /// matched on the name alone would claim the older run read what
    /// the newer one did.
    fn pinned_by(&self, name: &str, hash: &B3Hash) -> Vec<RunId> {
        self.skill_pins
            .get(&(name.to_owned(), *hash))
            .cloned()
            .unwrap_or_default()
    }
}

/// Which shelf a holding sits on, and where its document is.
///
/// A translation and not a judgement: the scan already said which shelf
/// it was reading, so nothing here derives a shelf from a path - a
/// derivation that could only be right about the shelves that happen to
/// sit in different places.
fn shelf_of(shelf: &city::Shelf) -> channels::SkillShelf {
    match shelf {
        city::Shelf::Library(addr) => channels::SkillShelf::Library(addr.clone()),
        city::Shelf::Building(addr) => channels::SkillShelf::Building(addr.clone()),
        city::Shelf::External { index, path } => channels::SkillShelf::External {
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

    /// One skill the way a shelf inside the city files its own: a
    /// section, then a document named after the skill.
    fn shelved(root: &Path, section: &str, name: &str, text: &str) {
        let dir = root.join(section);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.md")), text).unwrap();
    }

    /// One skill the way a shelf outside the city files it: a directory
    /// named after the skill, holding `SKILL.md`.
    fn external(root: &Path, name: &str, text: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), text).unwrap();
    }

    /// Every shelf a building reads reaches the page, and each row says
    /// which shelf it came from and where the document is. A skill on a
    /// shelf outside the city says which shelf of the configured list it
    /// is and its path from that shelf's root, because it has no address
    /// - which is the one thing the answer must not invent.
    #[test]
    fn every_shelf_reaches_the_page_with_the_place_it_sits_at() {
        use kernel::layout::CityLayout;

        let dir = tempfile::tempdir().unwrap();
        crate::assembly::init_city(dir.path()).unwrap();
        let lab = Address::parse("lab").unwrap();
        city::create_building(dir.path(), &lab, city::BuildingTemplate::Minimal).unwrap();
        let layout = CityLayout::new(dir.path());
        // The same name on both city shelves, so that the row also says
        // which of them won.
        shelved(
            &layout.library(),
            "utilities",
            "diffing",
            "The city's rule\n",
        );
        shelved(
            &layout.building_skills(&lab),
            "utilities",
            "diffing",
            "This lab's own rule\n",
        );
        shelved(
            &layout.library(),
            "utilities",
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
                // A Windows path in a TOML string would read as escapes;
                // the separators this city writes are forward slashes.
                elsewhere.display().to_string().replace('\\', "/")
            ),
        )
        .unwrap();

        let mut views = Views::new(dir.path());
        let channels::Answer::Skills(answer) = views.answer(&channels::Query::Skills {
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
            channels::SkillShelf::Building(at) => assert!(at.is_within(&lab), "{at}"),
            channels::SkillShelf::Library(_) | channels::SkillShelf::External { .. } => {
                panic!("the building's own copy is the nearer shelf")
            }
        }
        assert_eq!(named("diffing").disclosure, "This lab's own rule");
        match &named("kiln-firing").shelf {
            channels::SkillShelf::Library(at) => {
                assert!(at.is_reserved(), "a resident may not restock it: {at}")
            }
            channels::SkillShelf::Building(_) | channels::SkillShelf::External { .. } => {
                panic!("the city's stock is the library shelf")
            }
        }
        let outside = named("diagnosing-bugs");
        assert_eq!(
            outside.shelf,
            channels::SkillShelf::External {
                index: 0,
                path: "diagnosing-bugs/SKILL.md".to_owned(),
            },
            "an external row names its shelf and its path, and no address"
        );
        assert_eq!(outside.section, "", "that tree files no sections");
        assert_eq!(outside.disclosure, "Diagnose first");
    }
}
