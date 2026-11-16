// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

pub(super) mod endpoints;
mod environment;
pub(super) mod held;
mod probing;
pub(super) mod signing;

pub(super) use endpoints::Admission;

pub(super) const ENVIRONMENT_ENDPOINT: &str = "environment";

pub(super) struct Entered {
    pub(super) name: String,
    pub(super) base_url: String,
    pub(super) dialect: kernel::DialectKind,
    pub(super) credential: Credential,
    pub(super) tuning: gateway::EndpointTuning,
}

impl Entered {

    pub(super) fn resolved(mut self) -> Result<Entered, kernel::AxError> {
        let settled =
            gateway::normalise_entered(&self.base_url, gateway::DialectHint::of(self.dialect))?;
        self.dialect = settled.face();
        self.base_url = settled.base_url;
        Ok(self)
    }
}

pub(super) enum Credential {

    Absent { header: Option<String> },

    Key {
        reference: String,
        header: Option<String>,
    },
}

impl Credential {

    pub(super) fn entered(reference: Option<String>, header: Option<String>) -> Credential {
        match reference {
            None => Credential::Absent { header },
            Some(reference) => Credential::Key { reference, header },
        }
    }
}

pub(super) struct Chosen {
    pub(super) endpoint: String,
    pub(super) model: String,
    pub(super) tag: kernel::ModelTag,
}

pub(super) struct Stated {

    pub(super) context_tokens: Option<kernel::Window>,
    pub(super) max_output_tokens: Option<kernel::Ceiling>,

    pub(super) input: Option<kernel::InputKinds>,
}

impl Stated {

    pub(super) const fn new(
        context_tokens: Option<kernel::Window>,
        max_output_tokens: Option<kernel::Ceiling>,
        input: Option<kernel::InputKinds>,
    ) -> Self {
        Self {
            context_tokens,
            max_output_tokens,
            input,
        }
    }
}

pub(super) const PROBE_TIMEOUT_MS: u64 = 15_000;

pub(super) fn dialect_headers(dialect: kernel::DialectKind) -> Vec<(String, gateway::HeaderValue)> {
    match dialect {
        kernel::DialectKind::Anthropic => vec![(
            "anthropic-version".to_owned(),
            gateway::HeaderValue::Plain(ANTHROPIC_VERSION.to_owned()),
        )],
        kernel::DialectKind::OpenAi | kernel::DialectKind::OpenAiResponses => Vec::new(),
    }
}

pub(super) const ANTHROPIC_VERSION: &str = "2023-06-01";

pub(super) fn local_model_facts(model: &str) -> Result<gateway::ModelEntry, AxError> {
    let market = gateway::MarketSnapshot::builtin()?;
    let local = market.lookup("local").ok_or_else(|| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "read the model catalog",
            "the pinned catalog has no local row",
        )
        .with_recovery(
            "restore the `local` row in `gateway::market::MarketSnapshot::builtin`: \
             local inference is priced from it",
        )
    })?;
    Ok(gateway::ModelEntry {
        id: model.to_owned(),
        ..local.clone()
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
