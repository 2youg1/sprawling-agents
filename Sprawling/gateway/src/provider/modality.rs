// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub mod call;
pub mod embedding;
pub mod rerank;

use super::registry::ConnectionKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    Embedding,

    Rerank,
}

impl Modality {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Modality::Embedding => "embedding",
            Modality::Rerank => "rerank",
        }
    }

    pub const ALL: [Modality; 2] = [Modality::Embedding, Modality::Rerank];
}

impl ConnectionKind {
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
        }
    }

    #[must_use]
    pub fn url_for(self, base_url: &str, modality: Modality) -> Option<String> {
        Some(crate::router::join(base_url, self.path_for(modality)?))
    }
}

const EMBEDDINGS_PATH: &str = "embeddings";

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

    #[test]
    fn a_messages_endpoint_serves_no_vector_face() {
        for modality in Modality::ALL {
            assert_eq!(ConnectionKind::AnthropicNative.path_for(modality), None);
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
