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
mod environment;
mod probing;
pub(super) mod signing;

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
    pub(super) credential: Credential,
    pub(super) tuning: gateway::EndpointTuning,
}

/// The frame's tuning as the book keeps it.
///
/// Two readings happen here and nowhere else. **A zero is absence**: a
/// deadline no request can meet is what an empty box reaches the wire
/// as, and reading it as a figure would make every call fail instantly
/// for somebody who cleared a field. **`stream_idle_timeout_ms` becomes
/// a deadline for the whole streamed request**, which is what this
/// city's blocking transport can enforce; the wire keeps the name the
/// person's own `config.toml` uses, and `gateway` states what it does
/// with it (gateway-SPEC.md 8-NN).
///
/// A row with no name and a pointer with no path are dropped: a form
/// that keeps an empty row open while somebody types is a form whose
/// half-written rows must not reach a provider.
pub(super) fn tuning_of(wire: channels::EndpointTuning) -> gateway::EndpointTuning {
    let stated = |figure: Option<u64>| figure.filter(|ms| *ms > 0);
    gateway::EndpointTuning {
        label: wire
            .label
            .map(|given| given.trim().to_owned())
            .filter(|given| !given.is_empty()),
        timeout_ms: stated(wire.timeout_ms),
        request_max_retries: wire.request_max_retries,
        stream_deadline_ms: stated(wire.stream_idle_timeout_ms),
        extra_headers: wire
            .headers
            .into_iter()
            .map(|row| (row.name.trim().to_owned(), row.value))
            .filter(|(name, _)| !name.is_empty())
            .collect(),
        overrides: wire
            .overrides
            .into_iter()
            .map(|row| (row.pointer.trim().to_owned(), row.value))
            .filter(|(pointer, _)| pointer.starts_with('/'))
            .collect(),
        proxying: wire.proxying.unwrap_or_default(),
    }
}

/// How a credential proves itself to a provider.
///
/// Exhaustive rather than a reference beside a header name: a key and
/// a subscription token do not travel in the same header, and "a
/// subscription token in the header a key uses" is a 401 nobody can
/// read off a form.
pub(super) enum Credential {
    /// Nothing was enrolled.
    Absent,
    /// A key the person entered, as a `secret:realm/name` reference and
    /// never plaintext. The header is the compatible format's own
    /// answer unless the person named one.
    Key {
        reference: String,
        header: Option<String>,
    },
    /// What a login earned. Always `Authorization: Bearer`, whatever
    /// the compatible format does with keys: both first parties issue
    /// their subscription tokens that way.
    Subscription { reference: String },
}

impl Credential {
    /// What a wire command carries. A subscription token is never among
    /// it: that one is earned by a login inside this process.
    pub(super) fn entered(reference: Option<String>, header: Option<String>) -> Credential {
        match reference {
            None => Credential::Absent,
            Some(reference) => Credential::Key { reference, header },
        }
    }
}

/// Which model, at which endpoint, for which of the city's roles.
pub(super) struct Chosen {
    pub(super) endpoint: String,
    pub(super) model: String,
    pub(super) tag: kernel::ModelTag,
}

/// The two ceilings a model row states.
///
/// The pair travels together because a context window without an output
/// ceiling describes no model that can be called. A zero window and an
/// absent ceiling each mean "nobody stated this", and the catalogue's
/// figure is taken where the catalogue has a row for the model.
pub(super) struct Ceilings {
    pub(super) context_tokens: u64,
    pub(super) max_output_tokens: Option<kernel::Ceiling>,
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
