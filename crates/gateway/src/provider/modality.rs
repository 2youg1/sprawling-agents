// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an endpoint can be asked for besides a conversation, and where
//! it answers (shape 6 data face).
//!
//! **Retrieval needs two models this city has no home for.** An
//! embedding model turns text into a vector and a rerank model scores
//! passages against a question; neither holds a conversation, so
//! neither fits the chat face, and without a home for them each caller
//! would hang its own path off a base URL — which is the shape that
//! answers 404 on the first relay that spells it differently.
//!
//! One table answers both questions here: which connections serve a
//! modality, and under which path. A connection that does not serve one
//! says so by having no path, so a caller cannot reach a face that is
//! not there and has nothing to test separately.
//!
//! The path is half the answer and the bytes are the other half:
//! [`embedding`] and [`rerank`] hold the request each face takes and
//! the answer it gives, each cited to the description its vendor
//! publishes.

pub mod call;
pub mod embedding;
pub mod rerank;

use super::registry::{ConnectionKind, Family};

/// What an endpoint is asked to do beyond holding a conversation.
///
/// Chat is deliberately absent: the chat path belongs to the
/// registration that already calls it (`router::attached`), and naming
/// it again here would be a second home for a path this city sends
/// every turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    /// Text in, one vector out — `/embeddings` in the OpenAI shape.
    Embedding,
    /// A question and passages in, scores out — `/rerank` in the shape
    /// text-embeddings-inference and infinity both serve.
    Rerank,
}

impl Modality {
    /// The word this modality travels under in the ledger, on the wire,
    /// and in a person's configuration file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Modality::Embedding => "embedding",
            Modality::Rerank => "rerank",
        }
    }

    /// Both, in the order this module declares them. The data face a
    /// catalogue and a credential form are generated from, so that a
    /// third modality reaches every screen by being added here.
    pub const ALL: [Modality; 2] = [Modality::Embedding, Modality::Rerank];
}

impl ConnectionKind {
    /// The path this connection serves one modality under, hung off the
    /// registered base URL; `None` where the vendor serves no such
    /// face.
    ///
    /// The rows state what each vendor documents, and nothing is
    /// extrapolated from a neighbouring row:
    ///
    /// - Anthropic publishes no embeddings and no rerank API, so a
    ///   messages endpoint answers neither.
    /// - The OpenAI-compatible shape serves embeddings, and it is also
    ///   the shape the two local rerank servers answer in.
    /// - The responses face is OpenAI's own, whose base also serves
    ///   embeddings; rerank it does not serve.
    /// - A subscription harness sells a conversation. None of the four
    ///   documents a vector face, and calling one with a subscription
    ///   token would be a use its terms do not describe.
    #[must_use]
    pub const fn path_for(self, modality: Modality) -> Option<&'static str> {
        match self {
            ConnectionKind::OpenAiCompat => match modality {
                Modality::Embedding => Some(EMBEDDINGS_PATH),
                Modality::Rerank => Some(RERANK_PATH),
            },
            ConnectionKind::Responses => match modality {
                Modality::Embedding => Some(EMBEDDINGS_PATH),
                Modality::Rerank => None,
            },
            ConnectionKind::AnthropicNative => None,
            ConnectionKind::Harness(family) => match family {
                Family::Codex | Family::ClaudeCode | Family::GrokBuild | Family::KimiCli => None,
            },
        }
    }

    /// Where a modality is called for one registered base URL, or
    /// `None` when this connection does not serve it.
    ///
    /// The base and the path are joined by the one join this city has,
    /// the same one the chat face and the model list use.
    #[must_use]
    pub fn url_for(self, base_url: &str, modality: Modality) -> Option<String> {
        Some(crate::router::join(base_url, self.path_for(modality)?))
    }
}

/// OpenAI's own spelling, which every compatible server copied.
/// <https://platform.openai.com/docs/api-reference/embeddings>
const EMBEDDINGS_PATH: &str = "embeddings";

/// The path both local rerank servers answer under, relative to the
/// base a person registered.
/// <https://huggingface.co/docs/text-embeddings-inference/en/quick_tour>
const RERANK_PATH: &str = "rerank";

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
    fn a_compatible_endpoint_serves_both_faces_under_its_own_base() {
        let kind = ConnectionKind::OpenAiCompat;
        assert_eq!(
            kind.url_for("https://relay.example.test/v1", Modality::Embedding),
            Some("https://relay.example.test/v1/embeddings".to_owned())
        );
        assert_eq!(
            kind.url_for("https://relay.example.test/v1", Modality::Rerank),
            Some("https://relay.example.test/v1/rerank".to_owned())
        );
    }

    /// A face the vendor does not publish has no path, so a caller
    /// cannot build a URL for it at all.
    #[test]
    fn a_messages_endpoint_and_a_subscription_serve_no_vector_face() {
        for modality in Modality::ALL {
            assert_eq!(ConnectionKind::AnthropicNative.path_for(modality), None);
            for family in Family::ALL {
                assert_eq!(ConnectionKind::Harness(family).path_for(modality), None);
            }
        }
        assert_eq!(
            ConnectionKind::AnthropicNative
                .url_for("https://api.anthropic.com/v1", Modality::Embedding),
            None
        );
    }

    #[test]
    fn the_responses_face_serves_embeddings_and_no_rerank() {
        assert_eq!(
            ConnectionKind::Responses.path_for(Modality::Embedding),
            Some("embeddings")
        );
        assert_eq!(ConnectionKind::Responses.path_for(Modality::Rerank), None);
    }

    /// Every modality this city serves has a path to call and a shape
    /// to call it with. A modality with a path and no shape would be a
    /// URL nobody can write a body for.
    #[test]
    fn every_served_modality_has_both_a_path_and_a_wire_shape() {
        let served = ConnectionKind::OpenAiCompat;
        for modality in Modality::ALL {
            assert!(served.path_for(modality).is_some(), "{modality:?}");
        }
        let vectors = embedding::EmbeddingRequest::new("m".to_owned(), vec!["one".to_owned()])
            .expect("one text is a request");
        assert!(vectors.body().contains("encoding_format"));
        let ranking = rerank::RerankRequest::new("q".to_owned(), vec!["one".to_owned()])
            .expect("one passage is a request");
        assert!(ranking.body().contains("texts"));
    }

    #[test]
    fn every_modality_writes_one_word() {
        let mut words: Vec<&str> = Modality::ALL.iter().map(|one| one.as_str()).collect();
        words.sort_unstable();
        let written = words.len();
        words.dedup();
        assert_eq!(words.len(), written, "two modalities under one word");
    }
}
