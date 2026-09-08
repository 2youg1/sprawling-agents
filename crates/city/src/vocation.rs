// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether the residents at an address build or plan (city-SPEC.md
//! section 8-22).
//!
//! City Hall writes Markdown and plans; it does not build. That is what
//! the tool set assembled for a run at `hall` leaves out — `exec`,
//! `delegate` and `workshop` — and the decision is taken from the
//! address rather than from the wording of an identity file, because an
//! invariant a prompt is asked to keep is not an invariant.

use kernel::Address;
use kernel::consts_policy::HALL_BUILDING;

/// What the residents at an address are there to do. Exhaustive: a
/// third vocation is a third arm at every place that lays out a bench,
/// which is a compile error rather than a new building quietly falling
/// into the wrong branch of a bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vocation {
    /// It may run programs, hand work down, and route a build floor.
    Builds,
    /// It writes documents and plans, for buildings that build.
    Plans,
}

/// Which of the two an address's building is for.
///
/// Takes the building rather than the room: what a resident is for is a
/// property of the building it works in, and the room is where it keeps
/// its files. A caller holding a room address asks
/// [`crate::Building::of`] first, which is the one place a room becomes
/// a building.
#[must_use]
pub fn vocation_of(building: &Address) -> Vocation {
    if building.as_str() == HALL_BUILDING {
        Vocation::Plans
    } else {
        Vocation::Builds
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
    use kernel::Address;

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    #[test]
    fn city_hall_plans_and_every_other_building_builds() {
        assert_eq!(vocation_of(&addr("hall")), Vocation::Plans);
        assert_eq!(vocation_of(&addr("lab")), Vocation::Builds);
        assert_eq!(vocation_of(&addr("hallway")), Vocation::Builds);
    }
}
