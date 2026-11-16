// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Address;
use kernel::consts_policy::HALL_BUILDING;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vocation {
    Builds,

    Plans,
}

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
