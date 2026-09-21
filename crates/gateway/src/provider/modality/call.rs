// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One call to a vector face, end to end: the URL, the bytes out, the
//! answer read back, and the line the ledger keeps (shape 4 adapter).
//!
//! The shape of each face is `embedding`'s and `rerank`'s; which
//! connection serves a face is `super`'s table. What was missing was
//! the step between them, and it is missing in the direction that
//! matters: a caller left to write it would decide for itself whether
//! the endpoint's credential travels, which status is a failure, and
//! what a body that will not parse becomes. Those answers already have
//! one home - the endpoint transport - so a call goes through it and
//! this module supplies only the two things a vector face differs by:
//! which face of the connection it asks for, and the body.
//!
//! **A person's body overrides do not reach a vector face.** An
//! override is a JSON pointer into a conversation's body, applied by
//! creating the path it names; the same pointer against an embeddings
//! body would add a field no vendor reads, or worse, answer a question
//! the person asked about their chat. Extra headers do travel: they are
//! about the endpoint, not about one body.
//!
//! **Each call returns its own ledger line.** The record travels back
//! with the answer rather than being appended here, because a line
//! belongs to a run's history and this module has no history to write
//! to: the caller that holds the ledger appends what it was handed.
//!
//! **The credential is redeemed per call.** A redemption is not cached
//! by design - it exists for one operation and is dropped - so it is a
//! parameter of each call rather than a field of the face.

use kernel::event::record::{EmbeddingCalled, RerankCalled};
use kernel::{AxCode, AxError, Tokens};
use serde_json::Value;

use super::Modality;
use super::embedding::{EmbeddingRequest, Embeddings};
use super::rerank::{Ranking, RerankRequest};
use crate::endpoint::{Endpoint, EndpointConfig, Redemption};
use crate::router::AttachedEndpoint;

/// One attached endpoint's embeddings face, ready to call.
///
/// A type rather than a flag, so a rerank request cannot be sent to an
/// embeddings path by a caller that mixed two variables up.
pub struct Vectors<'a> {
    face: Face<'a>,
}

/// One attached endpoint's rerank face, ready to call.
pub struct Ranks<'a> {
    face: Face<'a>,
}

/// What both faces need to make one call.
struct Face<'a> {
    endpoint: &'a AttachedEndpoint,
    /// Where this face is called: the registered base and the path this
    /// connection serves the face under, composed once by [`super`]'s
    /// `url_for`, which owns both.
    url: String,
    /// The provider's own id for the model that answers this face,
    /// which is not the conversation's: an embedding model holds no
    /// conversation, and the id an endpoint is called with is its own
    /// fact.
    model: String,
}

impl<'a> Vectors<'a> {
    /// The embeddings face of one attached endpoint.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when this connection serves no embeddings
    /// face: the refusal names the connection and what to do instead,
    /// so a wrong model is not mistaken for a wrong registration.
    pub fn of(endpoint: &'a AttachedEndpoint, model: String) -> Result<Vectors<'a>, AxError> {
        Ok(Vectors {
            face: Face::of(endpoint, Modality::Embedding, model)?,
        })
    }

    /// Where this face is called.
    #[must_use]
    pub fn url(&self) -> String {
        self.face.url().to_owned()
    }

    /// One call, and the line that records it.
    ///
    /// # Errors
    /// `E_PROVIDER` when the transport fails, the provider refuses, or
    /// the answer is not the shape this face owes; the answer's own
    /// bytes are never quoted back.
    pub fn embed(
        &self,
        request: &EmbeddingRequest,
        redemption: Redemption,
    ) -> Result<(Embeddings, EmbeddingCalled), AxError> {
        let answer = self.face.post(request.body(), redemption)?;
        let read = Embeddings::parse(&answer, request)?;
        // The two counts are taken from what was asked and what came
        // back rather than from each other: an answer that returned
        // fewer vectors than it was given texts is the defect this line
        // exists to show.
        let record = EmbeddingCalled {
            model: self.face.model.clone(),
            inputs: u64::try_from(request.texts()).unwrap_or(u64::MAX),
            vectors: u64::try_from(read.vectors().len()).unwrap_or(u64::MAX),
            dimensions: request.dimensions().map(u64::from),
            prompt_tokens: read.prompt_tokens().map(Tokens::new),
        };
        Ok((read, record))
    }
}

impl<'a> Ranks<'a> {
    /// The rerank face of one attached endpoint.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when this connection serves no rerank face,
    /// as [`Vectors::of`] refuses.
    pub fn of(endpoint: &'a AttachedEndpoint, model: String) -> Result<Ranks<'a>, AxError> {
        Ok(Ranks {
            face: Face::of(endpoint, Modality::Rerank, model)?,
        })
    }

    /// Where this face is called.
    #[must_use]
    pub fn url(&self) -> String {
        self.face.url().to_owned()
    }

    /// One call, and the line that records it.
    ///
    /// The line carries the two counts and no usage: these faces report
    /// none, and `kernel::event::record::RerankCalled` is where that
    /// shape is stated.
    ///
    /// # Errors
    /// `E_PROVIDER` when the transport fails, the provider refuses, or
    /// the answer is not the shape this face owes.
    pub fn rank(
        &self,
        request: &RerankRequest,
        redemption: Redemption,
    ) -> Result<(Ranking, RerankCalled), AxError> {
        let answer = self.face.post(request.body(), redemption)?;
        let read = Ranking::parse(&answer, request)?;
        let record = RerankCalled {
            model: self.face.model.clone(),
            passages: u64::try_from(request.passages()).unwrap_or(u64::MAX),
            ranks: u64::try_from(read.ranks().len()).unwrap_or(u64::MAX),
        };
        Ok((read, record))
    }
}

impl<'a> Face<'a> {
    fn of(
        endpoint: &'a AttachedEndpoint,
        modality: Modality,
        model: String,
    ) -> Result<Face<'a>, AxError> {
        // The base and the path are joined by the one function that
        // owns both, so a face carries the address itself rather than a
        // path each caller would have to hang off a base again.
        let url = endpoint
            .connection_kind
            .url_for(&endpoint.base_url, modality)
            .ok_or_else(|| unserved(endpoint, modality))?;
        Ok(Face {
            endpoint,
            url,
            model,
        })
    }

    /// Where this face is called, composed when the face was built.
    fn url(&self) -> &str {
        &self.url
    }

    /// One POST, through the transport every other call leaves by.
    ///
    /// The endpoint is built per call rather than kept: it holds a
    /// connection pool and a redeemed credential, and a long-lived one
    /// would hold a secret for as long as a retrieval batch runs.
    fn post(&self, body: String, redemption: Redemption) -> Result<Value, AxError> {
        let tuning = &self.endpoint.tuning;
        let endpoint = Endpoint::new(
            EndpointConfig {
                base_url: self.url.clone(),
                dialect: self.endpoint.connection_kind.wire(),
                model: self.model.clone(),
                auth: self.endpoint.auth.clone(),
                extra_headers: tuning.extra_headers.clone(),
                // An override names a field of a conversation's body;
                // see this module's header.
                overrides: Vec::new(),
                timeout_ms: tuning.call_timeout_ms(),
                // A vector face answers in one body, so it is bounded by
                // the call deadline and not by a silence.
                stream_idle_timeout_ms: None,
                pricing: None,
                proxying: tuning.proxying,
            },
            redemption,
        )?;
        endpoint.post_bytes("application/json", body.into_bytes())
    }
}

/// The refusal for a face this endpoint does not serve.
///
/// Stated in terms of the connection rather than of a path, because
/// that is the fact the person can change: this connection is a
/// subscription login, or a conversation face, and neither sells a
/// vector.
fn unserved(endpoint: &AttachedEndpoint, modality: Modality) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "call a vector face",
        format!(
            "{} is connected as {}",
            endpoint.label(),
            endpoint.connection_kind.as_str()
        ),
    )
    .with_recovery(format!(
        "this connection serves no {} face; attach an endpoint that serves one and choose \
         the model for `{}` there",
        modality.as_str(),
        modality.as_str()
    ))
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
