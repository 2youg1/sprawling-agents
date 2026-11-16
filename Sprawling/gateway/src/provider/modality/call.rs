// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{EmbeddingCalled, RerankCalled};
use kernel::{AxCode, AxError, Tokens};
use serde_json::Value;

use super::Modality;
use super::embedding::{EmbeddingRequest, Embeddings};
use super::rerank::{Ranking, RerankRequest};
use crate::endpoint::{Endpoint, EndpointConfig, Redemption};
use crate::router::AttachedEndpoint;

pub struct Vectors<'a> {
    face: Face<'a>,
}

pub struct Ranks<'a> {
    face: Face<'a>,
}

struct Face<'a> {
    endpoint: &'a AttachedEndpoint,

    url: String,

    model: String,
}

impl<'a> Vectors<'a> {

    pub fn of(endpoint: &'a AttachedEndpoint, model: String) -> Result<Vectors<'a>, AxError> {
        Ok(Vectors {
            face: Face::of(endpoint, Modality::Embedding, model)?,
        })
    }

    #[must_use]
    pub fn url(&self) -> String {
        self.face.url().to_owned()
    }

    pub fn embed(
        &self,
        request: &EmbeddingRequest,
        redemption: Redemption,
    ) -> Result<(Embeddings, EmbeddingCalled), AxError> {
        let answer = self.face.post(request.body(), redemption)?;
        let read = Embeddings::parse(&answer, request)?;

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

    pub fn of(endpoint: &'a AttachedEndpoint, model: String) -> Result<Ranks<'a>, AxError> {
        Ok(Ranks {
            face: Face::of(endpoint, Modality::Rerank, model)?,
        })
    }

    #[must_use]
    pub fn url(&self) -> String {
        self.face.url().to_owned()
    }

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

    fn url(&self) -> &str {
        &self.url
    }

    fn post(&self, body: String, redemption: Redemption) -> Result<Value, AxError> {
        let tuning = &self.endpoint.tuning;
        let endpoint = Endpoint::new(
            EndpointConfig {
                base_url: self.url.clone(),
                dialect: self.endpoint.connection_kind.wire(),
                model: self.model.clone(),
                auth: self.endpoint.first_auth()?,
                extra_headers: tuning.extra_headers.clone(),

                overrides: Vec::new(),
                timeout_ms: tuning.call_timeout_ms(),

                stream_idle_timeout_ms: None,
                pricing: None,
                proxying: tuning.proxying,
            },
            redemption,
        )?;
        endpoint.post_bytes("application/json", body.into_bytes())
    }
}

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
