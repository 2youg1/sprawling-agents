// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError};

use crate::resident::Identity;

const BRING_HEADING: &str = "<bring>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Occupancy {
    Resident { bring: String },

    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbour {
    pub addr: Address,

    pub name: String,
    pub occupancy: Occupancy,

    pub waiting: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbourhood {
    building: Address,
    here: Vec<Neighbour>,
    buildings: Vec<Address>,
}

impl Neighbourhood {
    pub fn scan(
        city_root: &Path,
        building: &Address,
        me: &Address,
        waiting: &dyn Fn(&Address) -> u32,
    ) -> Result<Neighbourhood, AxError> {
        let mut here = Vec::new();
        let mut places = vec![building.clone()];
        places.extend(crate::room::all(city_root, building)?);
        for addr in places {
            if &addr == me {
                continue;
            }
            let identity = Identity::load(city_root, &addr)?;
            let occupancy = match identity {
                Identity::Resident(_) => Occupancy::Resident {
                    bring: bring_line(&String::from_utf8_lossy(&identity.segment_bytes())),
                },
                Identity::Ephemeral { .. } => Occupancy::Empty,
            };
            here.push(Neighbour {
                name: addr.name().to_owned(),
                waiting: waiting(&addr),
                addr,
                occupancy,
            });
        }
        Ok(Neighbourhood {
            building: building.clone(),
            here,
            buildings: crate::building::all(city_root)?,
        })
    }

    #[must_use]
    pub fn building(&self) -> &Address {
        &self.building
    }

    #[must_use]
    pub fn here(&self) -> &[Neighbour] {
        &self.here
    }

    #[must_use]
    pub fn buildings(&self) -> &[Address] {
        &self.buildings
    }

    #[must_use]
    pub fn residents(&self) -> u32 {
        let count = self
            .here
            .iter()
            .filter(|neighbour| matches!(neighbour.occupancy, Occupancy::Resident { .. }))
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

fn bring_line(text: &str) -> String {
    if let Some((_, tail)) = text.split_once(BRING_HEADING)
        && let Some(line) = section_line(tail)
    {
        return line;
    }
    first_prose(text).unwrap_or_default()
}

fn section_line(tail: &str) -> Option<String> {
    for line in tail.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with('<') {
            return None;
        }
        return Some(trimmed.to_owned());
    }
    None
}

fn first_prose(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with('>')
                && !line.starts_with('<')
        })
        .map(str::to_owned)
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
    use std::path::PathBuf;

    fn city() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    fn resident_at(root: &Path, at: &str, urbanite: &str) {
        let dir: PathBuf = root.join(at);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(kernel::layout::URBANITE_FILE), urbanite).unwrap();
    }

    fn empty_room(root: &Path, at: &str) {
        std::fs::create_dir_all(root.join(at)).unwrap();
    }

    #[test]
    fn a_run_sees_the_others_in_its_building_and_not_itself() {
        let dir = city();
        let root = dir.path();
        resident_at(
            root,
            "lab/mason",
            "# URBANITE.md\n\n## Bring them\n\nA kiln that will not fire.\n",
        );
        resident_at(
            root,
            "lab/scribe",
            "# URBANITE.md\n\nReads twice before writing once.\n",
        );
        empty_room(root, "lab/store");
        resident_at(root, "market/broker", "# URBANITE.md\n\nPrices things.\n");

        let seen = Neighbourhood::scan(root, &addr("lab"), &addr("lab/mason"), &|_| 0).unwrap();
        let names: Vec<&str> = seen
            .here()
            .iter()
            .map(|neighbour| neighbour.addr.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["lab", "lab/scribe", "lab/store"],
            "the building root is a place too, and a run is not its own neighbour"
        );
        assert_eq!(seen.residents(), 1, "only scribe stands anywhere here");
        assert_eq!(
            seen.here()[1].occupancy,
            Occupancy::Resident {
                bring: "Reads twice before writing once.".to_owned()
            },
            "with no `Bring them` section the first line of prose stands in"
        );
        assert_eq!(seen.here()[2].occupancy, Occupancy::Empty);
        assert_eq!(seen.here()[2].name, "store");
    }

    #[test]
    fn the_rest_of_the_city_is_names_and_nothing_else() {
        let dir = city();
        let root = dir.path();
        resident_at(root, "lab/mason", "# URBANITE.md\n\nFires kilns.\n");
        resident_at(root, "market/broker", "# URBANITE.md\n\nPrices things.\n");
        std::fs::create_dir_all(root.join(kernel::RESERVED_PREFIX)).unwrap();

        let seen = Neighbourhood::scan(root, &addr("lab"), &addr("lab/mason"), &|_| 0).unwrap();
        let buildings: Vec<&str> = seen
            .buildings()
            .iter()
            .map(kernel::Address::as_str)
            .collect();
        assert_eq!(
            buildings,
            vec!["lab", "market"],
            "the city's own account is not a building"
        );
        assert!(
            !seen
                .here()
                .iter()
                .any(|n| n.addr.as_str().starts_with("market")),
            "another building's residents are not in reach and so are not shown"
        );
    }

    #[test]
    fn a_buildings_own_filing_is_not_a_neighbour() {
        let dir = city();
        let root = dir.path();
        empty_room(root, "lab/notes");
        std::fs::create_dir_all(root.join("lab").join(kernel::RESERVED_PREFIX)).unwrap();
        std::fs::create_dir_all(root.join("lab").join(kernel::layout::ARCHIVE_DIR)).unwrap();

        let seen = Neighbourhood::scan(root, &addr("lab"), &addr("lab/notes"), &|_| 0).unwrap();
        let names: Vec<&str> = seen
            .here()
            .iter()
            .map(|neighbour| neighbour.addr.as_str())
            .collect();
        assert_eq!(names, vec!["lab"], "one place, and it is the building root");
    }

    #[test]
    fn the_line_comes_from_bring_them_when_the_resident_wrote_one() {
        let filled = "# URBANITE.md — mason\n\n> guidance the author left in place\n\n<who>\nA potter.\n</who>\n\n<bring>\nAnything that has to survive a firing.\n</bring>\n";
        assert_eq!(bring_line(filled), "Anything that has to survive a firing.");

        let unsectioned = "# URBANITE.md\n\n> guidance\n\nAsks rather than guesses.\n";
        assert_eq!(
            bring_line(unsectioned),
            "Asks rather than guesses.",
            "the template's guidance is not a description of anybody"
        );

        let sectionless_but_headed =
            "# URBANITE.md\n\n<bring>\n</bring>\n\n<who>\nA potter.\n</who>\n";
        assert_eq!(
            bring_line(sectionless_but_headed),
            "A potter.",
            "an empty section falls back rather than borrowing the next one's line"
        );

        assert_eq!(
            bring_line(""),
            "",
            "an empty description is a choice, not a defect"
        );
    }

    #[test]
    fn the_shipped_template_still_carries_the_section_this_reads() {
        let template = include_str!("../templates/URBANITE.md");
        assert!(
            template.contains(BRING_HEADING),
            "Sprawling/city/templates/URBANITE.md no longer has {BRING_HEADING}"
        );
    }
}
