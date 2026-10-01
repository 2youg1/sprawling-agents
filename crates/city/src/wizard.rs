// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Starting a city.
//!
//! This is a decision rather than an action. What a new city consists of
//! is answered here as a value; making the directories and writing the
//! events is the binary's work. That split is what lets "one instruction
//! builds a city" be tested without building one.
//!
//! Specified by `crates/city/spec/Wizard.lean` §8-10.

use kernel::{Address, AxCode, AxError, RESERVED_PREFIX};

use crate::building::BuildingTemplate;

/// What a directory already holds, seen from a city about to form in
/// it.
///
/// Exhaustive on purpose: "the directory is not empty" is not an answer
/// anybody can act on. Each arm names what happens next, so the person
/// who points at a folder they have been working in for a year is told
/// what will be laid down beside their work and what will not be
/// touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// Nothing here. A city forms and touches nothing, because there is
    /// nothing to touch.
    Empty,
    /// Work that is not a city. Forming one adds the reserved subtree
    /// and the city's own prompt beside it; the folders already here can
    /// become buildings, and nothing in them is read, moved or
    /// rewritten.
    Work {
        /// The top-level folders that could each become a building, in
        /// the order a listing gives them, sorted so two machines
        /// looking at one directory answer the same.
        adoptable: Vec<Address>,
        /// Loose files at the top level. Counted rather than listed:
        /// what a person needs to know is that they are there and that
        /// nothing will happen to them.
        loose: usize,
    },
    /// A city already. A second genesis over it would be a second answer
    /// to when this city began.
    AlreadyACity,
}

/// Reads a directory listing as a standing.
///
/// Takes the listing rather than the path, for the reason this whole
/// module takes values: what a city forming here would do is a decision,
/// and deciding it should not need a disk. `entries` is `(name, is_dir)`
/// as the caller read them; `has_history` is whether a ledger is already
/// there, which only the caller can see.
///
/// A name this city could not address - a dot directory, a name holding
/// a colon or a backslash - is counted as loose rather than offered as a
/// building: a building the address grammar cannot spell is one nothing
/// could ever dispatch to. A name with a space is spellable and is
/// offered, because the grammar takes it.
#[must_use]
pub fn survey(entries: &[(String, bool)], has_history: bool) -> Standing {
    if has_history {
        return Standing::AlreadyACity;
    }
    if entries.is_empty() {
        return Standing::Empty;
    }
    let mut adoptable = Vec::new();
    let mut loose: usize = 0;
    for (name, is_dir) in entries {
        if !is_dir || name.starts_with('.') {
            loose = loose.saturating_add(1);
            continue;
        }
        match Address::parse(name) {
            Ok(addr) if !addr.is_reserved() => adoptable.push(addr),
            _ => loose = loose.saturating_add(1),
        }
    }
    adoptable.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Standing::Work { adoptable, loose }
}

/// What `init` makes. Values only, so the shape of a new city can be
/// asserted without a filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityPlan {
    /// Directories to create, city-root relative, in creation order.
    dirs: Vec<Address>,
    /// City Hall, which every city has. Not an `Option`: a city without
    /// a hall is not something this version forms, and optionality would
    /// make "does this city have one" a question every caller answers
    /// again when it has one answer.
    hall: (Address, BuildingTemplate),
    /// The first building, if the instruction named one.
    first: Option<(Address, BuildingTemplate)>,
}

impl CityPlan {
    /// Plans a city from one instruction.
    ///
    /// `first` is the building to raise immediately. An empty city is
    /// legal — it is what an empty directory becomes — but a city with a
    /// first building is the ordinary case, and the wizard exists so
    /// that takes one instruction rather than three.
    ///
    /// # Errors
    /// Refuses a first building whose name is the reserved prefix, one
    /// that names a room rather than a building, and one whose template
    /// this version does not have.
    pub fn new(first: Option<(&str, &str)>) -> Result<CityPlan, AxError> {
        let hall_addr = Address::parse(kernel::consts_policy::HALL_BUILDING)?;
        let mut dirs = vec![Address::parse(RESERVED_PREFIX)?, hall_addr.clone()];
        let hall = (hall_addr, BuildingTemplate::Hall);
        let first = match first {
            None => None,
            Some((name, template)) => {
                let addr = Address::parse(name)?;
                if addr.is_reserved() {
                    return Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "raise the first building",
                        name.to_owned(),
                    )
                    .with_recovery(
                        "protected metadata names the city's own directories \
                         (`.sprawling`, `.git`); choose another name",
                    ));
                }
                if addr.as_str().contains('/') {
                    return Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "raise the first building",
                        name.to_owned(),
                    )
                    .with_recovery("name a building, not a room inside one"));
                }
                let template = BuildingTemplate::parse(template)?;
                dirs.push(addr.clone());
                Some((addr, template))
            }
        };
        Ok(CityPlan { dirs, hall, first })
    }

    #[must_use]
    pub fn dirs(&self) -> &[Address] {
        &self.dirs
    }

    /// City Hall, which every plan raises.
    #[must_use]
    pub fn hall(&self) -> &(Address, BuildingTemplate) {
        &self.hall
    }

    #[must_use]
    pub fn first(&self) -> Option<&(Address, BuildingTemplate)> {
        self.first.as_ref()
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
    use super::*;

    fn listing(entries: &[(&str, bool)]) -> Vec<(String, bool)> {
        entries
            .iter()
            .map(|(name, is_dir)| ((*name).to_owned(), *is_dir))
            .collect()
    }

    /// The case this exists for: somebody has been working in a folder
    /// for a year and points the city at it.
    #[test]
    fn a_folder_with_work_in_it_says_what_could_become_a_building() {
        let standing = survey(
            &listing(&[
                ("notes.md", false),
                ("parser", true),
                (".git", true),
                ("api", true),
                ("weird:name", true),
            ]),
            false,
        );
        let Standing::Work { adoptable, loose } = standing else {
            panic!("a folder with work in it is not empty and is not a city");
        };
        assert_eq!(
            adoptable
                .iter()
                .map(|addr| addr.as_str().to_owned())
                .collect::<Vec<String>>(),
            ["api", "parser"],
            "sorted, so two machines reading one directory answer the same"
        );
        assert_eq!(
            loose, 3,
            "a file, a dot directory and a name the address grammar cannot spell"
        );
    }

    #[test]
    fn an_empty_folder_and_a_city_are_different_answers() {
        assert_eq!(survey(&[], false), Standing::Empty);
        assert_eq!(
            survey(&listing(&[("parser", true)]), true),
            Standing::AlreadyACity,
            "history is what makes a directory a city, whatever else is in it"
        );
    }

    #[test]
    fn one_instruction_plans_a_city_with_somewhere_to_work_in_it() {
        let plan = CityPlan::new(Some(("lab", "minimal"))).unwrap();
        assert_eq!(plan.dirs()[0].as_str(), RESERVED_PREFIX);
        assert_eq!(
            plan.dirs()[1].as_str(),
            kernel::consts_policy::HALL_BUILDING
        );
        assert_eq!(plan.dirs()[2].as_str(), "lab");
        assert_eq!(plan.hall().1, BuildingTemplate::Hall);
        let (first, _) = plan.first().unwrap();
        assert_eq!(first.as_str(), "lab");
    }

    #[test]
    fn an_empty_city_is_legal_and_is_what_an_empty_directory_becomes() {
        let plan = CityPlan::new(None).unwrap();
        assert_eq!(
            plan.dirs().len(),
            2,
            "the reserved subtree and City Hall; a city always has both"
        );
        assert!(plan.first().is_none());
    }

    #[test]
    fn the_first_building_cannot_be_the_citys_own_subtree_or_a_room() {
        let reserved = CityPlan::new(Some((RESERVED_PREFIX, "minimal"))).unwrap_err();
        assert!(reserved.recovery().contains("another name"));
        let room = CityPlan::new(Some(("lab/room1", "minimal"))).unwrap_err();
        assert!(room.recovery().contains("not a room"));
    }
}
