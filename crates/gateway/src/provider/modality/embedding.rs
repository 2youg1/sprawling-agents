// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a `/embeddings` call looks like on the wire, in both
//! directions (shape 1 decision).
//!
//! **A vector face has one shape and several servers.** OpenAI
//! published it, and every compatible server — a relay, a local
//! text-embeddings-inference, an infinity — copied the same request
//! and the same answer. So the shape is written once here and cited to
//! OpenAI's own API description, rather than once per caller that
//! wants a vector.
//!
//! The request states `encoding_format` even though the vendor
//! defaults it: the reader below understands one encoding, and a
//! default the server could change is not a thing to read an answer
//! against.
//!
//! Sources, both read on 2026-09-21:
//! `openai/openai-openapi` `openapi.yaml`, schemas `CreateEmbeddingRequest`,
//! `CreateEmbeddingResponse` and `Embedding`;
//! `huggingface/text-embeddings-inference` `docs/openapi.json`, schemas
//! `OpenAICompatRequest` and `OpenAICompatResponse`, which is the same
//! shape served by a local model.

use kernel::{AxCode, AxError};
use serde_json::Value;

/// One call to a vector face.
#[derive(Debug)]
pub struct EmbeddingRequest {
    model: String,
    inputs: Vec<String>,
    dimensions: Option<u32>,
}

/// The encoding this city asks for and the only one it reads. OpenAI
/// also serves `base64`, which halves the bytes on the wire and would
/// need a decoder here before it may be asked for.
const ENCODING_FORMAT: &str = "float";

impl EmbeddingRequest {
    /// One request for one or more texts.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when no text was given, or when one of them is
    /// empty: OpenAI's description states the input "cannot be an
    /// empty string", so the refusal happens here rather than as a 400
    /// a person has to read the provider's words to understand.
    pub fn new(model: String, inputs: Vec<String>) -> Result<EmbeddingRequest, AxError> {
        if inputs.is_empty() || inputs.iter().any(String::is_empty) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "ask for an embedding",
                "the request carries no text to embed",
            )
            .with_recovery(
                "send at least one non-empty text: an embedding of nothing is not a \
                 vector the provider will return",
            ));
        }
        Ok(EmbeddingRequest {
            model,
            inputs,
            dimensions: None,
        })
    }

    /// Ask the model to shorten its vectors. Supported by the models
    /// whose documentation says so, and refused by the rest, which is
    /// the provider's answer to give rather than this city's.
    #[must_use]
    pub fn with_dimensions(mut self, dimensions: u32) -> EmbeddingRequest {
        self.dimensions = Some(dimensions);
        self
    }

    /// How many texts this request carries, which is how many vectors
    /// its answer must hold.
    #[must_use]
    pub fn texts(&self) -> usize {
        self.inputs.len()
    }

    /// The width this request asks for, where it asks for one. What the
    /// ledger line records, because a stored vector is only comparable
    /// with one of the same width.
    #[must_use]
    pub const fn dimensions(&self) -> Option<u32> {
        self.dimensions
    }

    /// The JSON body, in the field order the vendor documents.
    #[must_use]
    pub fn body(&self) -> String {
        let mut body = serde_json::json!({
            "model": self.model,
            "input": self.inputs,
            "encoding_format": ENCODING_FORMAT,
        });
        if let Some(dimensions) = self.dimensions
            && let Some(map) = body.as_object_mut()
        {
            map.insert("dimensions".to_owned(), Value::from(dimensions));
        }
        body.to_string()
    }
}

/// What a vector face answered: one vector per text, in the order the
/// texts were sent.
#[derive(Debug)]
pub struct Embeddings {
    vectors: Vec<Vec<f64>>,
    /// Which model answered, where the provider named one. A local
    /// server routinely names none, and `None` says that rather than
    /// an empty name.
    model: Option<String>,
    /// What the provider counted, where it counted. Absent and zero
    /// are different answers, so they are different values.
    prompt_tokens: Option<u64>,
}

impl Embeddings {
    /// Read an answer against the request that asked for it.
    ///
    /// The `index` field carries the order, and this reader uses it
    /// rather than the order the items happen to arrive in: OpenAI
    /// documents the field as "the index of the embedding in the list
    /// of embeddings", and a caller pairing vector 3 with text 1 has
    /// built a retrieval index that returns the wrong passage and says
    /// nothing about it.
    ///
    /// # Errors
    /// `E_PROVIDER` when the answer holds a different number of
    /// vectors than the request had texts, when an index is repeated
    /// or out of range, or when a vector holds something that is not a
    /// number.
    pub fn parse(answer: &Value, asked: &EmbeddingRequest) -> Result<Embeddings, AxError> {
        let data = answer
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("the answer states no `data` array"))?;
        if data.len() != asked.texts() {
            return Err(unreadable(&format!(
                "{} texts were sent and {} vectors came back",
                asked.texts(),
                data.len()
            )));
        }
        let mut slots: Vec<Option<Vec<f64>>> = vec![None; data.len()];
        for item in data {
            let index = item
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| unreadable("a vector came back without a readable `index`"))?;
            let slot = slots
                .get_mut(index)
                .ok_or_else(|| unreadable("a vector came back under an index nobody asked for"))?;
            if slot.is_some() {
                return Err(unreadable("two vectors came back under one index"));
            }
            *slot = Some(vector_at(item)?);
        }
        let vectors = slots
            .into_iter()
            .collect::<Option<Vec<Vec<f64>>>>()
            .ok_or_else(|| unreadable("one of the texts came back with no vector"))?;
        Ok(Embeddings {
            vectors,
            model: answer
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_owned),
            prompt_tokens: answer
                .get("usage")
                .and_then(|usage| usage.get("prompt_tokens"))
                .and_then(Value::as_u64),
        })
    }

    /// The vectors, in the order of the texts that produced them.
    ///
    /// Held as 64-bit although the wire states 32-bit: widening is
    /// exact, narrowing is not, and a city that bans `as` does not
    /// need a rounding rule to carry a number it only stores and
    /// compares.
    #[must_use]
    pub fn vectors(&self) -> &[Vec<f64>] {
        &self.vectors
    }

    /// Which model answered, as the provider named it. A relay may
    /// serve a different model than the one that was asked for, and
    /// this is where it says so.
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// What the provider counted for this call, where it counted at
    /// all. A caller settling a bill needs the difference between "no
    /// tokens" and "no statement".
    #[must_use]
    pub const fn prompt_tokens(&self) -> Option<u64> {
        self.prompt_tokens
    }
}

/// One item's vector, refusing anything that is not a list of numbers.
fn vector_at(item: &Value) -> Result<Vec<f64>, AxError> {
    let numbers = item
        .get("embedding")
        .and_then(Value::as_array)
        .ok_or_else(|| unreadable("a vector came back without an `embedding` list"))?;
    let mut vector = Vec::with_capacity(numbers.len());
    for number in numbers {
        let value = number
            .as_f64()
            .ok_or_else(|| unreadable("an embedding holds something that is not a number"))?;
        vector.push(value);
    }
    Ok(vector)
}

/// An answer this version cannot read, stated without quoting it.
fn unreadable(detail: &str) -> AxError {
    AxError::failure(
        AxCode::Provider,
        "read an embedding answer",
        detail.to_owned(),
    )
    .with_recovery(
        "check that this endpoint serves the OpenAI-compatible embeddings face: a \
             vector index that pairs a passage with another passage's vector returns \
             wrong answers silently",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn asked() -> EmbeddingRequest {
        EmbeddingRequest::new(
            "text-embedding-3-small".to_owned(),
            vec!["one".to_owned(), "two".to_owned()],
        )
        .unwrap()
    }

    #[test]
    fn the_request_states_the_encoding_it_will_read() {
        let body: Value = serde_json::from_str(&asked().body()).unwrap();
        assert_eq!(body["encoding_format"], "float");
        assert_eq!(body["input"][1], "two");
        assert!(body.get("dimensions").is_none());
        let shortened: Value = serde_json::from_str(&asked().with_dimensions(256).body()).unwrap();
        assert_eq!(shortened["dimensions"], 256);
    }

    #[test]
    fn an_empty_text_is_refused_before_the_provider_refuses_it() {
        let refusal = EmbeddingRequest::new("m".to_owned(), vec![String::new()]).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::InvalidArgs);
        assert!(EmbeddingRequest::new("m".to_owned(), Vec::new()).is_err());
    }

    /// The order on the wire is not the order of the texts, and the
    /// `index` field is what says so.
    #[test]
    fn vectors_are_ordered_by_the_index_the_provider_stated() {
        let answer = serde_json::json!({
            "object": "list",
            "model": "text-embedding-3-small",
            "data": [
                {"object": "embedding", "index": 1, "embedding": [0.5, 0.25]},
                {"object": "embedding", "index": 0, "embedding": [1.0, 0.0]},
            ],
            "usage": {"prompt_tokens": 7, "total_tokens": 7},
        });
        let read = Embeddings::parse(&answer, &asked()).unwrap();
        assert_eq!(read.vectors()[0], vec![1.0_f64, 0.0_f64]);
        assert_eq!(read.vectors()[1], vec![0.5_f64, 0.25_f64]);
        assert_eq!(read.model(), Some("text-embedding-3-small"));
        assert_eq!(read.prompt_tokens(), Some(7));
    }

    #[test]
    fn an_answer_that_does_not_match_the_request_is_refused() {
        let short = serde_json::json!({
            "data": [{"object": "embedding", "index": 0, "embedding": [1.0]}],
        });
        let refusal = Embeddings::parse(&short, &asked()).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::Provider);
        assert!(refusal.subject().contains("2 texts were sent"));

        let repeated = serde_json::json!({
            "data": [
                {"object": "embedding", "index": 0, "embedding": [1.0]},
                {"object": "embedding", "index": 0, "embedding": [2.0]},
            ],
        });
        assert!(Embeddings::parse(&repeated, &asked()).is_err());

        let not_numbers = serde_json::json!({
            "data": [
                {"object": "embedding", "index": 0, "embedding": ["x"]},
                {"object": "embedding", "index": 1, "embedding": [1.0]},
            ],
        });
        assert!(Embeddings::parse(&not_numbers, &asked()).is_err());
    }

    /// A local text-embeddings-inference server states no usage and no
    /// model it was not given; those two absences are facts about that
    /// server rather than a failure to read its answer.
    #[test]
    fn a_local_server_that_counts_nothing_is_still_read() {
        let answer = serde_json::json!({
            "object": "list",
            "data": [
                {"object": "embedding", "index": 0, "embedding": [0.1]},
                {"object": "embedding", "index": 1, "embedding": [0.2]},
            ],
        });
        let read = Embeddings::parse(&answer, &asked()).unwrap();
        assert_eq!(read.prompt_tokens(), None, "unstated is not zero");
        assert_eq!(read.model(), None);
        assert_eq!(read.vectors().len(), 2);
    }
}
