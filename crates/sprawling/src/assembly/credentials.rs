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
pub(super) mod subscription;

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

impl Entered {
    /// What a person pasted, resolved into the base URL this city calls
    /// and the shape it answers in.
    ///
    /// The one place a typed address becomes a called one. Probing and
    /// attaching both arrive here, so the two cannot reach different
    /// hosts from the same text \u2014 which is what B-02 was: the form
    /// probed one URL and the book recorded another, and the second one
    /// 404ed. A URL whose path already names a face decides the shape,
    /// because a pasted URL is evidence and a toggle left on its default
    /// is not.
    ///
    /// # Errors
    /// When the text carries a scheme this city cannot call, or no host
    /// to call at all.
    pub(super) fn resolved(mut self) -> Result<Entered, kernel::AxError> {
        let hint = match self.dialect {
            kernel::DialectKind::Anthropic => gateway::DialectHint::Messages,
            kernel::DialectKind::OpenAi => gateway::DialectHint::Chat,
        };
        let settled = gateway::normalise_entered(&self.base_url, hint)?;
        self.dialect = match settled.dialect {
            gateway::DialectHint::Messages => kernel::DialectKind::Anthropic,
            // `Responses` has no registration of its own until the wire
            // carries one (roadmap 4.5). Until then it is called the way
            // every other OpenAI-shaped endpoint is, which is what the
            // person chose when they picked that group.
            gateway::DialectHint::Chat
            | gateway::DialectHint::Responses
            | gateway::DialectHint::Unset => kernel::DialectKind::OpenAi,
        };
        self.base_url = settled.base_url;
        Ok(self)
    }
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
/// # Errors
/// `E_CONFIG_INVALID` naming the header whose value reads as a
/// credential without being a vault reference.
pub(super) fn tuning_of(
    wire: channels::EndpointTuning,
) -> Result<gateway::EndpointTuning, AxError> {
    let stated = |figure: Option<u64>| figure.filter(|ms| *ms > 0);
    let mut extra_headers = Vec::new();
    for row in wire.headers {
        let name = row.name.trim().to_owned();
        if name.is_empty() {
            continue;
        }
        extra_headers.push((
            name.clone(),
            gateway::HeaderValue::parse(&name, &row.value)?,
        ));
    }
    Ok(gateway::EndpointTuning {
        label: wire
            .label
            .map(|given| given.trim().to_owned())
            .filter(|given| !given.is_empty()),
        timeout_ms: stated(wire.timeout_ms),
        request_max_retries: wire
            .request_max_retries
            .map_or(gateway::Retries::UntilHalted, gateway::Retries::AtMost),
        stream_idle_timeout_ms: stated(wire.stream_idle_timeout_ms),
        extra_headers,
        overrides: wire
            .overrides
            .into_iter()
            .map(|row| (row.pointer.trim().to_owned(), row.value))
            .filter(|(pointer, _)| pointer.starts_with('/'))
            .collect(),
        proxying: wire.proxying.unwrap_or_default(),
    })
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
pub(super) fn dialect_headers(dialect: kernel::DialectKind) -> Vec<(String, gateway::HeaderValue)> {
    match dialect {
        kernel::DialectKind::Anthropic => vec![(
            "anthropic-version".to_owned(),
            gateway::HeaderValue::Plain(ANTHROPIC_VERSION.to_owned()),
        )],
        kernel::DialectKind::OpenAi => Vec::new(),
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

/// What one tag already points at, when it points at this same model
/// behind this same endpoint.
///
/// The book is the one statement of what this city registered, so an
/// earlier figure is read back out of it rather than held beside it.
/// Read by endpoint and model id rather than by tag alone: pointing a
/// tag at another model must not carry the old model's ceiling onto the
/// new one (sprawling-SPEC.md 8-71).
pub(super) fn registered_as(
    book: &gateway::EndpointBook,
    tag: kernel::ModelTag,
    endpoint: &str,
    model: &str,
) -> Option<gateway::ModelEntry> {
    book.choices()
        .find(|(held, at, entry)| *held == tag && *at == endpoint && entry.id == model)
        .map(|(_, _, entry)| entry.clone())
}

/// The catalog's `local` row under the name a local server serves it.
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
