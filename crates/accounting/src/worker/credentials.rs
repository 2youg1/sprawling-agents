// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which keys this city holds, and what it may call.
//!
//! The values and the readings live here; `held` is what the worker
//! keeps (the book and the vault), `signing` the vault's use,
//! `endpoints` what may be called.

use kernel::{AxCode, AxError};

mod endpoints;
mod environment;
pub(super) mod held;
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

impl Entered {
    /// What a person pasted, resolved into the base URL this city calls
    /// and the shape it answers in.
    ///
    /// The one place a typed address becomes a called one. Probing and
    /// attaching both arrive here, so the two cannot reach different
    /// hosts from the same text; otherwise the form could probe one URL
    /// while the book records another, and the second one 404s.
    /// A URL whose path already names a face decides the shape,
    /// because a pasted URL is evidence and a toggle left on its default
    /// is not.
    ///
    /// # Errors
    /// When the text carries a scheme this city cannot call, or no host
    /// to call at all.
    pub(super) fn resolved(mut self) -> Result<Entered, kernel::AxError> {
        let settled =
            gateway::normalise_entered(&self.base_url, gateway::DialectHint::of(self.dialect))?;
        // A shape nobody settled is the chat face, the one every
        // compatible server answers.
        self.dialect = settled
            .dialect
            .dialect()
            .unwrap_or(kernel::DialectKind::OpenAi);
        self.base_url = settled.base_url;
        Ok(self)
    }
}

/// How a credential proves itself to a provider.
///
/// Exhaustive rather than an optional reference: "nothing entered this
/// time" keeps the key the city already holds, which an absent
/// reference beside a header name cannot say.
pub(super) enum Credential {
    /// No key was entered this time, which is not the same as no key:
    /// the city keeps what it has for this endpoint, and an empty box
    /// never removes one (`crates/sprawling/Spec.lean` §8-81). The
    /// header is the one the person named for a key, which still
    /// decides how an archived reference travels.
    Absent { header: Option<String> },
    /// A key the person entered, as a `secret:realm/name` reference and
    /// never plaintext. The header is the compatible format's own
    /// answer unless the person named one.
    Key {
        reference: String,
        header: Option<String>,
    },
}

impl Credential {
    /// What a wire command carries.
    pub(super) fn entered(reference: Option<String>, header: Option<String>) -> Credential {
        match reference {
            None => Credential::Absent { header },
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

/// The three facts about a model that no model list returns, as a
/// person stated them.
///
/// They travel together because a model row states all three: a context
/// window without an output ceiling describes no model that can be
/// called, and what it accepts decides whether a picture may be sent at
/// all. Each absent means "nobody stated this", and the ladder that
/// fact climbs answers instead.
pub(super) struct Stated {
    /// `None` when nobody stated one. Zero is unrepresentable here:
    /// a window of zero and a window nobody registered would be the same
    /// byte, and the reminder would read every session as full.
    pub(super) context_tokens: Option<kernel::Window>,
    pub(super) max_output_tokens: Option<kernel::Ceiling>,
    /// The first rung of `gateway::accepted_input` (gateway D16).
    pub(super) input: Option<kernel::InputKinds>,
}

impl Stated {
    /// The three facts in the order `SelectModel` carries them; each
    /// has its own type, so two of them cannot trade places.
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
        kernel::DialectKind::OpenAi | kernel::DialectKind::OpenAiResponses => Vec::new(),
    }
}

/// The Messages API version this build speaks. Pinned rather than
/// omitted: the provider treats a missing version as an error, and a
/// floating one would change the wire under a replay.
/// <https://platform.claude.com/docs/en/api/messages>
pub(super) const ANTHROPIC_VERSION: &str = "2023-06-01";

/// What one tag already points at, when it points at this same model
/// behind this same endpoint.
///
/// The book is the one statement of what this city registered, so an
/// earlier figure is read back out of it rather than held beside it.
/// Read by endpoint and model id rather than by tag alone: pointing a
/// tag at another model must not carry the old model's ceiling onto the
/// new one (`crates/sprawling/Spec.lean` §8-71).
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
