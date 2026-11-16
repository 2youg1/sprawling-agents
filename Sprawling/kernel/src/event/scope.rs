// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::error::{AxCode, AxError};

const CITY: &str = "city";

const BUILDING: &str = "building";

const WORKSHOP: &str = "workshop";

const AT: char = ':';

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    City,

    Building(Address),

    Workshop(Address),
}

impl Scope {
    #[must_use]
    pub fn covers(&self, addr: &Address) -> bool {
        match self {
            Scope::City => true,
            Scope::Building(scope) | Scope::Workshop(scope) => addr.is_within(scope),
        }
    }

    pub fn parse(raw: &str) -> Result<Scope, AxError> {
        if raw == CITY {
            return Ok(Scope::City);
        }
        let unknown = || {
            AxError::failure(AxCode::WireMismatch, "read which scope a line names", raw)
                .with_recovery(
                    "open this city with the build that wrote its history: this one knows \
                     `city`, `building:<address>` and `workshop:<address>`",
                )
        };
        let (kind, addr) = raw.split_once(AT).ok_or_else(unknown)?;
        let addr = Address::parse(addr)?;
        match kind {
            BUILDING => Ok(Scope::Building(addr)),
            WORKSHOP => Ok(Scope::Workshop(addr)),
            _ => Err(unknown()),
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scope::City => f.write_str(CITY),
            Scope::Building(addr) => write!(f, "{BUILDING}{AT}{addr}", addr = addr.as_str()),
            Scope::Workshop(addr) => write!(f, "{WORKSHOP}{AT}{addr}", addr = addr.as_str()),
        }
    }
}

impl Serialize for Scope {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Scope {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Scope::parse(&raw).map_err(serde::de::Error::custom)
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

    #[test]
    fn the_three_scopes_write_the_words_the_histories_already_hold() {
        let building = Scope::Building(Address::parse("lab").unwrap());
        let workshop = Scope::Workshop(Address::parse("lab/bench").unwrap());
        assert_eq!(Scope::City.to_string(), "city");
        assert_eq!(building.to_string(), "building:lab");
        assert_eq!(workshop.to_string(), "workshop:lab/bench");
        for scope in [Scope::City, building, workshop] {
            assert_eq!(Scope::parse(&scope.to_string()).unwrap(), scope);
            assert_eq!(
                serde_json::to_string(&scope).unwrap(),
                format!("\"{scope}\"")
            );
        }
    }

    #[test]
    fn a_scope_reaches_what_is_inside_it_and_nothing_else() {
        let inside = Address::parse("lab/bench.md").unwrap();
        let elsewhere = Address::parse("hall/clerk.md").unwrap();
        let lab = Scope::Building(Address::parse("lab").unwrap());
        assert!(Scope::City.covers(&inside));
        assert!(lab.covers(&inside));
        assert!(!lab.covers(&elsewhere));
    }

    #[test]
    fn a_word_naming_no_scope_is_refused_rather_than_guessed() {
        for raw in ["lab", "borough:lab", "building:"] {
            assert!(Scope::parse(raw).is_err(), "{raw} should not read");
        }
    }
}
