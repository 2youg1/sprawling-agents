// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::approval::{ApprovalId, Autonomy, ClusterKey, Ruling};
use crate::error::{AxCode, AxError};
use crate::event::scope::Scope;
use crate::locator::B3Hash;
use crate::registry::ResidentId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalResolved {
    pub id: ApprovalId,

    pub verdict: Ruling,

    pub cluster: ClusterKey,
}

pub mod autonomy_word {
    use super::{Autonomy, AxCode, AxError, ResidentId};

    const DELEGATE: &str = "delegate";

    const OWNER: &str = "owner";

    const AT: char = ':';

    #[must_use]
    pub fn spell(autonomy: &Autonomy) -> String {
        match autonomy {
            Autonomy::Owner => OWNER.to_owned(),
            Autonomy::Delegate(resident) => {
                format!("{DELEGATE}{AT}{resident}", resident = resident.as_str())
            }
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AutonomyChanged {
    #[serde(default = "city_wide")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub scope: Scope,
    #[serde(with = "autonomy_word")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub autonomy: Autonomy,
}

fn city_wide() -> Scope {
    Scope::City
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Admittance {
    Halted,

    Released,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityHalted {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub scope: Scope,
    pub state: Admittance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GovernedDocumentWritten {
    pub which: String,

    pub bytes: usize,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub naming: Option<B3Hash>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub at: Option<Address>,

    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RulesChanged {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub scope: Scope,

    pub which: GoverningDocument,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub before: Option<B3Hash>,

    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub after: B3Hash,

    pub bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GoverningDocument {
    #[serde(rename = "RULES.toml")]
    Rules,

    #[serde(rename = "CONFIG.toml")]
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SpineDocumentWritten {
    pub building: String,

    pub which: String,

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
            naming: None,
            at: None,
            removed: false,
        })
        .unwrap();
        assert_eq!(
            serde_json::to_string(written.as_map()).unwrap(),
            r#"{"bytes":12,"which":"MAYOR.md"}"#
        );
    }

    #[test]
    fn a_rules_change_carries_the_file_name_and_two_digests_never_the_text() {
        let opening = RulesChanged {
            scope: Scope::Building(Address::parse("lab").unwrap()),
            which: GoverningDocument::Rules,
            before: None,
            after: B3Hash::digest(b"new"),
            bytes: 3,
        };
        let payload = Payload::of(&opening).unwrap();
        let text = serde_json::to_string(payload.as_map()).unwrap();
        assert_eq!(
            text,
            format!(
                r#"{{"after":"{}","bytes":3,"scope":"building:lab","which":"RULES.toml"}}"#,
                B3Hash::digest(b"new")
            )
        );
        assert_eq!(payload.read::<RulesChanged>().unwrap(), opening);
        let moved = RulesChanged {
            which: GoverningDocument::Config,
            before: Some(B3Hash::digest(b"old")),
            ..opening
        };
        assert_eq!(
            Payload::of(&moved).unwrap().read::<RulesChanged>().unwrap(),
            moved
        );
    }
}
