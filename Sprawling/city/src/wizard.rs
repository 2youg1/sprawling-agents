// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, RESERVED_PREFIX};

use crate::building::BuildingTemplate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    Empty,

    Work {
        adoptable: Vec<Address>,

        loose: usize,
    },

    AlreadyACity,
}

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
        match is_dir
            .then(|| crate::building::names_a_building(name))
            .flatten()
        {
            Some(addr) => adoptable.push(addr),
            None => loose = loose.saturating_add(1),
        }
    }
    adoptable.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Standing::Work { adoptable, loose }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityPlan {
    dirs: Vec<Address>,

    hall: (Address, BuildingTemplate),

    first: Option<(Address, BuildingTemplate)>,
}

impl CityPlan {
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
