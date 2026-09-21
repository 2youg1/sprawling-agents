// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who a ledger line is attributed to.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::error::{AxCode, AxError};

/// The city's own desk, as every ledger line it writes spells it.
const CITY: &str = "city";

/// A human being, as a ledger line spells them. One word for all of
/// them: the ledger says an instruction came from outside the machine,
/// and which person it was is the session's business rather than the
/// history's.
const PERSON: &str = "person";

/// The three parties a ledger line can be attributed to.
///
/// Every line of every history carries one of these, and until this
/// type existed each writer spelled its own: six call sites wrote the
/// literal `"city"` and a resident wrote its address, so a seventh
/// spelling was one keystroke away and nothing would have refused it.
///
/// **The wire form is what the histories already hold.** The city is
/// `city`, a person is `person`, and a resident is its address, exactly
/// as before; no ledger on disk has to be re-spelled for this type to
/// read it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Who {
    /// The city's desk: genesis, dispatch, truncation, relaying.
    City,
    /// A person, through a client.
    Person,
    /// A resident of a building, at the address it is known by.
    Resident(Address),
}

impl Who {
    /// A resident's attribution.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for an address spelled `city` or `person`:
    /// those two words are the other two parties on this wire, and a
    /// resident answering to one of them would make a line's author
    /// unreadable.
    pub fn resident(addr: Address) -> Result<Who, AxError> {
        if addr.as_str() == CITY || addr.as_str() == PERSON {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "attribute a ledger line",
                addr.as_str(),
            )
            .with_recovery(
                "give this resident an address other than `city` or `person`: both \
                 words name another party the ledger attributes lines to",
            ));
        }
        Ok(Who::Resident(addr))
    }

    /// The word a ledger line carries.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Who::City => CITY,
            Who::Person => PERSON,
            Who::Resident(addr) => addr.as_str(),
        }
    }

    /// Reads back what [`Who::as_str`] wrote.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a word that is neither reserved nor a
    /// canonical address.
    pub fn parse(raw: &str) -> Result<Who, AxError> {
        match raw {
            CITY => Ok(Who::City),
            PERSON => Ok(Who::Person),
            other => Ok(Who::Resident(Address::parse(other)?)),
        }
    }
}

impl fmt::Display for Who {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Who {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Who {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Who::parse(&raw).map_err(serde::de::Error::custom)
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
    fn the_three_parties_write_the_words_the_histories_already_hold() {
        let resident = Who::resident(Address::parse("lobby/scribe.md").unwrap()).unwrap();
        assert_eq!(Who::City.to_string(), "city");
        assert_eq!(Who::Person.to_string(), "person");
        assert_eq!(resident.to_string(), "lobby/scribe.md");
        for party in [Who::City, Who::Person, resident] {
            assert_eq!(Who::parse(party.as_str()).unwrap(), party);
            assert_eq!(
                serde_json::to_string(&party).unwrap(),
                format!("\"{party}\"")
            );
        }
    }

    #[test]
    fn a_resident_may_not_answer_to_either_reserved_word() {
        for reserved in ["city", "person"] {
            let err = Who::resident(Address::parse(reserved).unwrap()).unwrap_err();
            assert_eq!(err.code(), &AxCode::InvalidArgs);
        }
    }

    #[test]
    fn a_word_that_is_not_an_address_is_refused_rather_than_guessed() {
        let err = Who::parse("../elsewhere").unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
    }
}
