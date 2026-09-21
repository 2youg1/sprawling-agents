// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person's standing decisions record: who may answer, what is
//! shut, how a question was answered, and which governing document was
//! written.

use serde::{Deserialize, Serialize};

use crate::approval::{ApprovalId, ClusterKey, Ruling};

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

/// `autonomy_changed`: who answers the Approval Inbox for a scope from
/// now on.
///
/// Both values are the words the ledger has always held: the scope as
/// `sprawling::assembly::naming::scope_name` spells it, the autonomy as
/// `autonomy_name` does. Those two spellings are still that module's,
/// and moving them into this crate is leaf 7.8 — it waits on the four
/// readers outside this family that spell them by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AutonomyChanged {
    pub scope: String,
    pub autonomy: String,
}

/// `city_halted`: one scope stopped taking work, or started again.
///
/// One kind for both directions, because halting and releasing are one
/// fact changing value; `state` carries the word
/// `sprawling::assembly::folds::Admission` spells, which stays that
/// type's to own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityHalted {
    pub scope: String,
    pub state: String,
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
            scope: "building:lab".to_owned(),
            state: "halted".to_owned(),
        })
        .unwrap();
        assert_eq!(
            serde_json::to_string(halted.as_map()).unwrap(),
            r#"{"scope":"building:lab","state":"halted"}"#
        );
        let autonomy = Payload::of(&AutonomyChanged {
            scope: "city".to_owned(),
            autonomy: "delegate:hall/clerk.md".to_owned(),
        })
        .unwrap();
        assert_eq!(
            serde_json::to_string(autonomy.as_map()).unwrap(),
            r#"{"autonomy":"delegate:hall/clerk.md","scope":"city"}"#
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
