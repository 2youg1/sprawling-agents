// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person's standing decisions record: who may answer, what is
//! shut, how a question was answered, and which governing document was
//! written.

use serde::{Deserialize, Serialize};

use crate::approval::{ApprovalId, Autonomy, ClusterKey, Ruling};
use crate::error::{AxCode, AxError};
use crate::event::scope::Scope;
use crate::registry::ResidentId;

/// `approval_resolved`: how one inbox item was answered.
///
/// The verdict is the [`Ruling`] itself rather than a word formatted
/// from it. Its two spellings used to come out of `{verdict:?}`
/// lowercased at the writer and be compared against the literal
/// `"allow"` at one reader and `"deny"` at another, so a word neither
/// recognised was read as a refusal on one side and as consent on the
/// other. A [`Ruling`] a reader cannot parse is now a refusal to read
/// the line at all.
///
/// No field defaults: all three keys have been written on every
/// `approval_resolved` line since the kind existed, and an answer
/// missing its verdict is a line to refuse rather than to guess at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalResolved {
    /// The item answered.
    pub id: ApprovalId,
    /// What the answerer decided.
    pub verdict: Ruling,
    /// The group the person was shown and answered as one. A resumed
    /// run may act again within it without asking, which is why the
    /// group travels with the answer rather than being re-derived.
    pub cluster: ClusterKey,
}

/// The word the ledger writes for who answers a scope's Approval
/// Inbox, and its reader.
///
/// `owner` is the person; `delegate:<resident>` is the resident they
/// appointed, whose address travels in the value so a replay knows who
/// was appointed. The two halves sit together because they are one
/// spelling: a writer and a reader in different modules is how an
/// autonomy written by one build came back as `Owner` in the next.
pub mod autonomy_word {
    use super::{Autonomy, AxCode, AxError, ResidentId};

    /// The word before an appointed resident's address.
    const DELEGATE: &str = "delegate";

    /// The person who owns the city.
    const OWNER: &str = "owner";

    /// What separates the appointment from the resident it names.
    const AT: char = ':';

    /// The word a line carries.
    #[must_use]
    pub fn spell(autonomy: &Autonomy) -> String {
        match autonomy {
            Autonomy::Owner => OWNER.to_owned(),
            Autonomy::Delegate(resident) => {
                format!("{DELEGATE}{AT}{resident}", resident = resident.as_str())
            }
        }
    }

    /// Reads back what [`spell`] wrote.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` for any other word. It used to fall back to
    /// the person, which reads as the strict side and is not: a city
    /// whose history appointed a delegate this build cannot read would
    /// have shown the person questions the delegate was answering, and
    /// said nothing about the line it could not read.
    pub fn read(word: &str) -> Result<Autonomy, AxError> {
        let unreadable = |detail: &str| {
            AxError::failure(AxCode::WireMismatch, "read who answers for a scope", detail)
                .with_recovery(
                    "open this city with the build that wrote its history: this one knows \
                     `owner` and `delegate:<resident>`",
                )
        };
        if word == OWNER {
            return Ok(Autonomy::Owner);
        }
        match word.split_once(AT) {
            Some((DELEGATE, resident)) => ResidentId::new(resident)
                .map(Autonomy::Delegate)
                .ok_or_else(|| unreadable(resident)),
            Some(_) | None => Err(unreadable(word)),
        }
    }

    pub(super) fn serialize<S: serde::Serializer>(
        autonomy: &Autonomy,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&spell(autonomy))
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Autonomy, D::Error> {
        let word = <String as serde::Deserialize>::deserialize(deserializer)?;
        read(&word).map_err(serde::de::Error::custom)
    }
}

/// `autonomy_changed`: who answers the Approval Inbox for a scope from
/// now on.
///
/// Both values are the words the ledger has always held: the scope as
/// [`Scope`] spells it, the appointment as [`autonomy_word`] does. Both
/// spellings were a `format!` in `sprawling::assembly` and a
/// `split_once(':')` at each reader until this struct became the only
/// way in and out of the line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AutonomyChanged {
    /// What the appointment applies to. A line written before the key
    /// existed carries no scope and appointed the answerer for the
    /// whole city, which is what [`city_wide`] reads it as.
    #[serde(default = "city_wide")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub scope: Scope,
    #[serde(with = "autonomy_word")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub autonomy: Autonomy,
}

/// What an `autonomy_changed` line with no scope applies to.
fn city_wide() -> Scope {
    Scope::City
}

/// Whether a scope is shut or open.
///
/// The two words a `city_halted` record carries are spelled here and
/// nowhere else. They used to be two string constants compared by hand
/// at four places, and the two folds disagreed about an unrecognised
/// word: one read it as a release, the other ignored the line
/// (sprawling-SPEC.md 8-74).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Admittance {
    /// Nothing may be dispatched into this scope.
    Halted,
    /// The scope takes work again.
    Released,
}

/// `city_halted`: one scope stopped taking work, or started again.
///
/// One kind for both directions, because halting and releasing are one
/// fact changing value, and a second kind would let a reader see a
/// release with no halt before it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityHalted {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub scope: Scope,
    pub state: Admittance,
}

/// `governed_document_written`: which of the three documents a person
/// wrote, and how long it now is.
///
/// Never the text. The document is on disk and readable there, and a
/// copy here would be a second authority for words a person edits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GovernedDocumentWritten {
    /// The document's file name, as `city::Governed::file` gives it.
    pub which: String,
    /// The length of what was written, in bytes.
    pub bytes: usize,
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
    use crate::address::Address;
    use crate::approval::ApprovalClass;
    use crate::event::Payload;

    /// The bytes these structs write are the bytes the hand-written
    /// writers wrote, key for key and word for word.
    #[test]
    fn an_answered_item_writes_the_keys_the_histories_already_hold() {
        let answered = ApprovalResolved {
            id: ApprovalId::new("ap-7").unwrap(),
            verdict: Ruling::Allow,
            cluster: ClusterKey {
                class: ApprovalClass::Question,
                detail: "ask the person".to_owned(),
            },
        };
        let payload = Payload::of(&answered).unwrap();
        assert_eq!(
            serde_json::to_string(payload.as_map()).unwrap(),
            r#"{"cluster":{"class":"question","detail":"ask the person"},"id":"ap-7","verdict":"allow"}"#
        );
        assert_eq!(payload.read::<ApprovalResolved>().unwrap(), answered);
        let denied = ApprovalResolved {
            verdict: Ruling::Deny,
            ..answered
        };
        assert!(
            serde_json::to_string(Payload::of(&denied).unwrap().as_map())
                .unwrap()
                .contains(r#""verdict":"deny""#)
        );
    }

    /// The two words that used to fall back to the strict side, and the
    /// scope that used to be read with `split_once(':')`.
    #[test]
    fn an_appointment_this_build_cannot_read_is_refused_rather_than_defaulted() {
        let line = serde_json::json!({"scope": "city", "autonomy": "anybody"});
        let payload: Payload = serde_json::from_value(line).unwrap();
        let err = payload.read::<AutonomyChanged>().unwrap_err();
        assert_eq!(err.code(), &AxCode::WireMismatch);
        let line = serde_json::json!({"scope": "lab", "state": "halted"});
        let payload: Payload = serde_json::from_value(line).unwrap();
        assert!(payload.read::<CityHalted>().is_err());
    }

    #[test]
    fn a_verdict_this_build_does_not_know_is_refused_rather_than_defaulted() {
        let line = serde_json::json!({
            "id": "ap-7",
            "verdict": "maybe",
            "cluster": {"class": "question", "detail": "d"},
        });
        let payload: Payload = serde_json::from_value(line).unwrap();
        assert!(payload.read::<ApprovalResolved>().is_err());
    }

    #[test]
    fn the_three_standing_records_keep_their_keys() {
        let halted = Payload::of(&CityHalted {
            scope: Scope::Building(Address::parse("lab").unwrap()),
            state: Admittance::Halted,
        })
        .unwrap();
        assert_eq!(
            serde_json::to_string(halted.as_map()).unwrap(),
            r#"{"scope":"building:lab","state":"halted"}"#
        );
        let autonomy = Payload::of(&AutonomyChanged {
            scope: Scope::City,
            autonomy: Autonomy::Delegate(ResidentId::new("hall/clerk.md").unwrap()),
        })
        .unwrap();
        assert_eq!(
            serde_json::to_string(autonomy.as_map()).unwrap(),
            r#"{"autonomy":"delegate:hall/clerk.md","scope":"city"}"#
        );
        assert_eq!(
            autonomy.read::<AutonomyChanged>().unwrap().autonomy,
            Autonomy::Delegate(ResidentId::new("hall/clerk.md").unwrap())
        );
        let written = Payload::of(&GovernedDocumentWritten {
            which: "MAYOR.md".to_owned(),
            bytes: 12,
        })
        .unwrap();
        assert_eq!(
            serde_json::to_string(written.as_map()).unwrap(),
            r#"{"bytes":12,"which":"MAYOR.md"}"#
        );
    }
}
