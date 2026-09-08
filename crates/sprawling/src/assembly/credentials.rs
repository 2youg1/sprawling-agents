// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city can sign in as, and what it may call.
//!
//! The values and the readings live here; `signing` holds the login
//! and the vault, `endpoints` what may be called.

use kernel::{AxCode, AxError};

mod endpoints;
mod signing;

/// The name the environment-configured endpoint is attached under, so a
/// person reading the settings page can see where it came from.
pub(super) const ENVIRONMENT_ENDPOINT: &str = "environment";

/// What a person entered to reach a provider.
///
/// Five values that are read together and never chosen independently:
/// probing an endpoint, attaching it and describing it to the ledger are
/// three readings of one form, and passing them side by side gave three
/// chances for the probe and the attachment to disagree about what they
/// were talking to.
pub(super) struct Entered {
    pub(super) name: String,
    pub(super) base_url: String,
    pub(super) dialect: kernel::DialectKind,
    /// A `secret:realm/name` reference, never plaintext.
    pub(super) secret: Option<String>,
    /// The header the credential travels in, when the provider wants one
    /// that is not `Authorization: Bearer`.
    pub(super) auth_header: Option<String>,
}

/// Which model, at which endpoint, for which of the city's roles.
pub(super) struct Chosen {
    pub(super) endpoint: String,
    pub(super) model: String,
    pub(super) tag: kernel::ModelTag,
}

/// The two ceilings a model row states.
///
/// Zero in either field means "take the catalogue's figure": a person
/// choosing a model on a form has no business typing a context window,
/// and the pair travels together because a context window without an
/// output ceiling describes no model that can be called.
pub(super) struct Ceilings {
    pub(super) context_tokens: u64,
    pub(super) max_output_tokens: u64,
}

/// How long a probe may take. Short: a person is watching the settings
/// page, and an endpoint that cannot answer in this time is one they
/// want to hear about rather than wait for.
pub(super) const PROBE_TIMEOUT_MS: u64 = 15_000;

/// The headers a dialect requires beyond the credential.
pub(super) fn dialect_headers(dialect: kernel::DialectKind) -> Vec<(String, String)> {
    match dialect {
        kernel::DialectKind::Anthropic => {
            vec![("anthropic-version".to_owned(), ANTHROPIC_VERSION.to_owned())]
        }
        _ => Vec::new(),
    }
}

/// The Messages API version this build speaks. Pinned rather than
/// omitted: the provider treats a missing version as an error, and a
/// floating one would change the wire under a replay.
/// <https://platform.claude.com/docs/en/api/messages>
pub(super) const ANTHROPIC_VERSION: &str = "2023-06-01";

pub(super) fn poisoned_vault() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "reach the vault",
        "the vault lock is poisoned",
    )
    .with_recovery("restart the server; enrolled credentials are unaffected")
}

pub(super) fn dialect_of(provider: &str) -> Result<kernel::DialectKind, AxError> {
    match provider {
        "anthropic" => Ok(kernel::DialectKind::Anthropic),
        "openai" => Ok(kernel::DialectKind::OpenAi),
        other => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "choose a dialect for a provider",
            other.to_owned(),
        )
        .with_recovery("attach this provider by hand and state its dialect")),
    }
}

/// The catalog's `local` row under the name a local server serves it.
pub(super) fn local_model_facts(model: &str) -> Result<gateway::ModelEntry, AxError> {
    let market = gateway::MarketSnapshot::builtin();
    let local = market.lookup("local").ok_or_else(|| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "read the model catalog",
            "the pinned catalog has no local row",
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
    reason = "test code"
)]
mod tests;
