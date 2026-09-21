// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two faces of an attached endpoint that are not the conversation:
//! what one embedding or rerank call asked, and what came back.
//!
//! One line per settled call rather than a called/returned pair, because
//! neither answer is part of a conversation: what a reader asks of these
//! lines is what was bought - which model read how many texts, and how
//! many vectors or ranks came back for them. The vectors themselves are
//! absent for the reason `ModelCalled` carries no request body: they are
//! derived values a replay recomputes from the recorded inputs, and a
//! second copy here would be a second answer to the same question.

use serde::{Deserialize, Serialize};

use crate::budget::Tokens;

/// `embedding_called`: one call to an attached endpoint's embeddings
/// face.
///
/// The two counts are recorded rather than one being derived from the
/// other: an answer that returned fewer vectors than it was given texts
/// is a retrieval defect, and this is the line that shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EmbeddingCalled {
    /// The endpoint's own model id, as the request spelled it.
    pub model: String,
    /// How many texts the call asked to embed.
    pub inputs: u64,
    /// How many vectors the answer carried.
    pub vectors: u64,
    /// The width the caller asked for. Absent when it named none, which
    /// is not the same fact as a width of zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u64>,
    /// What the provider said it read. Absent when it said nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<Tokens>,
}

/// `rerank_called`: one call to an attached endpoint's rerank face.
///
/// The order the service returned is the answer and is not repeated
/// here; what this line adds is the size of what was asked and of what
/// came back, so a rerank that quietly scored half the passages can be
/// found afterwards. The line states no token count, because the rerank
/// faces this city writes to report none, and a column nothing can fill
/// is a promise to the ledger's reader that is never kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RerankCalled {
    /// The endpoint's own model id, as the request spelled it.
    pub model: String,
    /// How many passages the call offered to rank.
    pub passages: u64,
    /// How many ranks the answer carried.
    pub ranks: u64,
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
    use crate::event::Payload;

    /// The keys are the contract: a ledger already written cannot be
    /// re-spelled, so the shape of a line is pinned here.
    #[test]
    fn an_embedding_line_keeps_the_four_keys_and_omits_what_was_not_stated() {
        let call = EmbeddingCalled {
            model: "bge-m3".to_owned(),
            inputs: 12,
            vectors: 12,
            dimensions: None,
            prompt_tokens: Some(Tokens::new(77)),
        };
        let payload = Payload::of(&call).unwrap();
        assert_eq!(
            serde_json::to_string(&payload).unwrap(),
            "{\"inputs\":12,\"model\":\"bge-m3\",\"prompt_tokens\":77,\"vectors\":12}"
        );
        assert_eq!(
            serde_json::to_string(
                &Payload::of(&EmbeddingCalled {
                    dimensions: Some(1024),
                    ..call.clone()
                })
                .unwrap()
            )
            .unwrap(),
            "{\"dimensions\":1024,\"inputs\":12,\"model\":\"bge-m3\",\"prompt_tokens\":77,\"vectors\":12}"
        );
        assert_eq!(payload.read::<EmbeddingCalled>().unwrap(), call);
    }

    #[test]
    fn a_rerank_line_carries_the_two_counts_and_no_token_column() {
        let call = RerankCalled {
            model: "bge-reranker".to_owned(),
            passages: 40,
            ranks: 40,
        };
        let payload = Payload::of(&call).unwrap();
        assert_eq!(
            serde_json::to_string(&payload).unwrap(),
            "{\"model\":\"bge-reranker\",\"passages\":40,\"ranks\":40}"
        );
        assert_eq!(payload.read::<RerankCalled>().unwrap(), call);
    }
}
