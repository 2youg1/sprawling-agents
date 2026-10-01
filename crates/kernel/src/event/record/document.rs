// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A document saved through the page, and the three lines of a
//! proposal's life (kernel-SPEC.md section 8-83).
//!
//! **The save carries two digests and never the text.** The document is
//! on disk; the line says which document moved, from which version to
//! which. A proposal is different: its two texts are what a run said,
//! which no file keeps, so they travel in the line and a reopened city
//! folds an undecided card back from the Ledger alone.
//!
//! The span is two integers rather than `documents::Span`, because the
//! kernel does not depend on `documents`; a reader turns them back into
//! a span through the one constructor that refuses a reversed pair.

use serde::{Deserialize, Serialize};

use crate::{Address, B3Hash};

/// `document_written`: one save of one document landed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentWritten {
    /// Which document.
    pub at: Address,
    /// The version the save was made on.
    pub baseline: B3Hash,
    /// The version it left, which is the next save's baseline.
    pub version: B3Hash,
    /// How long the new version is, not how much changed.
    pub bytes: u64,
}

/// `proposal_offered`: a run suggests replacing one stretch of one
/// version with other text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalOffered {
    pub doc: Address,
    pub baseline: B3Hash,
    /// The half-open byte interval of the baseline the suggestion is
    /// about.
    pub start: u64,
    pub end: u64,
    /// The text that interval holds, in the baseline's encoding.
    pub before: String,
    /// The text suggested in its place.
    pub after: String,
}

/// `proposal_decided`: a person's verdicts on one card. A changed
/// sentence the verdicts do not name is rejected, so an empty list
/// rejects the whole card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalDecided {
    pub proposal: B3Hash,
    pub verdicts: Vec<SliceVerdict>,
}

/// A person's verdict on one sentence of a card, by its place on the
/// card counted from zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SliceVerdict {
    pub slice: u32,
    pub verdict: Verdict,
}

/// Take the sentence's change, or take an inserted sentence after
/// rewriting it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Verdict {
    Accept,
    Amend { text: String },
}

/// `proposal_withdrawn`: the run that offered a card took it back while
/// it was still open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalWithdrawn {
    pub proposal: B3Hash,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    /// Each line's bytes, so a ledger written by this build reads back
    /// in the next one.
    #[test]
    fn each_document_line_spells_its_keys_once() {
        let doc = Address::parse("lab/notes.md").unwrap();
        let zero = B3Hash::from_bytes([0; 32]);
        let hex = "0".repeat(64);
        let cases: [(Payload, String); 4] = [
            (
                Payload::of(&DocumentWritten {
                    at: doc.clone(),
                    baseline: zero,
                    version: zero,
                    bytes: 3,
                })
                .unwrap(),
                format!(
                    "{{\"at\":\"lab/notes.md\",\"baseline\":\"{hex}\",\"bytes\":3,\"version\":\"{hex}\"}}"
                ),
            ),
            (
                Payload::of(&ProposalOffered {
                    doc,
                    baseline: zero,
                    start: 1,
                    end: 2,
                    before: "a".to_owned(),
                    after: "b".to_owned(),
                })
                .unwrap(),
                format!(
                    "{{\"after\":\"b\",\"baseline\":\"{hex}\",\"before\":\"a\",\"doc\":\"lab/notes.md\",\"end\":2,\"start\":1}}"
                ),
            ),
            (
                Payload::of(&ProposalDecided {
                    proposal: zero,
                    verdicts: vec![
                        SliceVerdict {
                            slice: 1,
                            verdict: Verdict::Accept,
                        },
                        SliceVerdict {
                            slice: 2,
                            verdict: Verdict::Amend {
                                text: "c".to_owned(),
                            },
                        },
                    ],
                })
                .unwrap(),
                format!(
                    "{{\"proposal\":\"{hex}\",\"verdicts\":[{{\"slice\":1,\"verdict\":\"accept\"}},{{\"slice\":2,\"verdict\":{{\"amend\":{{\"text\":\"c\"}}}}}}]}}"
                ),
            ),
            (
                Payload::of(&ProposalWithdrawn { proposal: zero }).unwrap(),
                format!("{{\"proposal\":\"{hex}\"}}"),
            ),
        ];
        for (payload, wire) in cases {
            assert_eq!(serde_json::to_string(&payload).unwrap(), wire);
            let back: Payload = serde_json::from_str(&wire).unwrap();
            assert_eq!(back, payload);
        }
    }
}
