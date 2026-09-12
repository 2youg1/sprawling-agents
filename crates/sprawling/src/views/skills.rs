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
    /// Both shelves one building reads from, with the runs that pinned
    /// each holding.
    ///
    /// `None` for a building whose shelves will not scan, which is a
    /// broken installation rather than an empty one: `Unavailable` says
    /// the view could not look, and an empty list would say this
    /// building can do nothing.
    pub(super) fn skills_answer(&self, building: &Address) -> Option<channels::SkillsAnswer> {
        let shelves = city::Library::scan(&self.city_root, Some(building)).ok()?;
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
                shelf: shelf_of(building, &holding.addr),
                at: holding.addr.clone(),
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
    /// A pin naming no hash is skipped rather than filed under a guess:
    /// what a page answers with this is "which runs read these exact
    /// bytes", and a pin with no hash cannot answer it.
    pub(super) fn fold_skill_pins(&mut self, record: &EventRecord) {
        let Some(pins) = record
            .data()
            .as_map()
            .get("skills")
            .and_then(serde_json::Value::as_array)
        else {
            return;
        };
        for pin in pins {
            let (Some(name), Some(hash)) = (
                pin.get("name").and_then(serde_json::Value::as_str),
                pin.get("hash")
                    .cloned()
                    .and_then(|raw| serde_json::from_value::<B3Hash>(raw).ok()),
            ) else {
                continue;
            };
            let runs = self.skill_pins.entry((name.to_owned(), hash)).or_default();
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

/// Which shelf a holding sits on, read off its address.
///
/// A holding inside the building is the building's own; everything else
/// came from the city's stock. Derived from the address rather than
/// carried on the scan, because the address is what the scan already
/// computed and a second field would be a second answer to it.
fn shelf_of(building: &Address, at: &Address) -> channels::SkillShelf {
    if at.is_within(building) {
        return channels::SkillShelf::Building;
    }
    channels::SkillShelf::Library
}
