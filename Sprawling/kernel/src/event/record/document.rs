// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::{Address, B3Hash};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentWritten {
    pub at: Address,

    pub baseline: B3Hash,

    pub version: B3Hash,

    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalOffered {
    pub doc: Address,
    pub baseline: B3Hash,

    pub start: u64,
    pub end: u64,

    pub before: String,

    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalDecided {
    pub proposal: B3Hash,
    pub verdicts: Vec<SliceVerdict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SliceVerdict {
    pub slice: u32,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Verdict {
    Accept,
    Amend { text: String },
}

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
