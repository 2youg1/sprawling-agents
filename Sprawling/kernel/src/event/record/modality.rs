// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::budget::Tokens;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EmbeddingCalled {

    pub model: String,

    pub inputs: u64,

    pub vectors: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<Tokens>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RerankCalled {

    pub model: String,

    pub passages: u64,

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
