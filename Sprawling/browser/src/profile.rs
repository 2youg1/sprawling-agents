// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, RESERVED_PREFIX};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    path: Address,
}

pub const PROFILES_DIR: &str = "browser-profiles";

impl Profile {
    pub fn of(building: &Address) -> Result<Profile, AxError> {
        if building.is_reserved() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "place a browser profile",
                building.as_str().to_owned(),
            )
            .with_recovery("name a building; the city's own subtree is not one"));
        }
        if building.as_str().contains('/') {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "place a browser profile",
                building.as_str().to_owned(),
            )
            .with_recovery(
                "name the building, not a room inside it: a login belongs to the project",
            ));
        }
        let path = Address::parse(&format!(
            "{RESERVED_PREFIX}/{PROFILES_DIR}/{}",
            building.as_str()
        ))?;
        Ok(Profile { path })
    }

    #[must_use]
    pub fn path(&self) -> &Address {
        &self.path
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

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    #[test]
    fn two_buildings_never_share_what_the_browser_remembers() {
        let lab = Profile::of(&addr("lab")).unwrap();
        let mill = Profile::of(&addr("mill")).unwrap();
        assert_ne!(lab, mill);
        assert!(
            !lab.path().is_within(mill.path()) && !mill.path().is_within(lab.path()),
            "one building's cookie jar is not inside another's"
        );
    }

    #[test]
    fn a_profile_sits_where_no_write_domain_reaches() {
        let profile = Profile::of(&addr("lab")).unwrap();
        assert!(
            profile.path().is_reserved(),
            "a run that could edit its own cookie jar could grant itself an account"
        );
    }

    #[test]
    fn a_room_has_no_login_of_its_own() {
        let err = Profile::of(&addr("lab/room1")).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.recovery().contains("belongs to the project"));
    }

    #[test]
    fn the_citys_own_subtree_is_not_a_building() {
        assert!(Profile::of(&addr(RESERVED_PREFIX)).is_err());
    }
}
