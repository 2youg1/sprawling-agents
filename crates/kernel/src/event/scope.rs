// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a standing decision applies to, as the ledger spells it.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::error::{AxCode, AxError};

/// The whole city, as a `city_halted` or `autonomy_changed` line spells
/// it.
const CITY: &str = "city";

/// The word before the address of a building.
const BUILDING: &str = "building";

/// The word before the address of a workshop.
const WORKSHOP: &str = "workshop";

/// What separates the kind of a scope from the address it names.
const AT: char = ':';

/// What a halt, a release or an autonomy change applies to.
///
/// **The wire form is what the histories already hold**: `city`,
/// `building:<addr>`, `workshop:<addr>`. That spelling used to be a
/// `format!` at one writer and a `split_once(':')` at each reader, so a
/// scope written one way and read another was one keystroke away. The
/// three words are here, the parse is here, and both ends of the ledger
/// call this type.
///
/// A word this build does not know is refused rather than read as some
/// other scope: a city that read an unknown scope as its own would
/// report itself shut, and one that read it as nothing would dispatch
/// into a scope a person stopped.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    /// Everything this city holds.
    City,
    /// One building and what is inside it.
    Building(Address),
    /// One workshop and what is inside it.
    Workshop(Address),
}

impl Scope {
    /// Whether a standing decision on this scope reaches an address.
    ///
    /// The city reaches everything; a building and a workshop reach
    /// what is within them, by the containment [`Address::is_within`]
    /// already decides, so "inside" means one thing in this city.
    #[must_use]
    pub fn covers(&self, addr: &Address) -> bool {
        match self {
            Scope::City => true,
            Scope::Building(scope) | Scope::Workshop(scope) => addr.is_within(scope),
        }
    }

    /// Reads back what [`fmt::Display`] wrote.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` for a word that names none of the three
    /// shapes, and whatever [`Address::parse`] says about an address
    /// this city cannot hold.
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
