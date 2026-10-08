// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an endpoint said about one model it serves.

use kernel::{Ceiling, model::Window};
use serde::{Deserialize, Serialize};

/// One model an endpoint serves, with what the endpoint said about it.
///
/// **An endpoint's list is more than bare ids.** Everything the
/// provider states beside each id — the window, the ceiling, what the
/// model accepts, what it costs — is what a person chooses a model by.
/// This is that statement, carried to the page that shows the list.
///
/// **Every field from `id` to `output_price` is what the upstream said,
/// not what this city concluded.** Absence means the row said nothing; it
/// never means zero, and it is never filled in from the preset table here.
/// The ladder that picks a figure — the person's own entry, then the
/// upstream's statement, then the preset table, then the policy default
/// — runs where the call is made, and a summary that had already
/// climbed it would be a second answer to which figure won.
///
/// The last two fields are the city's answers, each from its one
/// authority: `thinking` names the rung it came from, and `canonical` is
/// `gateway::provider::identity`'s (`crates/wire/spec/Answer/Endpoints.lean` §8-92).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelFactsSummary {
    pub id: String,
    /// How many tokens this model reads in one call.
    pub context_tokens: Option<Window>,
    /// How many tokens it may write back.
    pub max_output_tokens: Option<Ceiling>,
    /// What it accepts, in the provider's own words (`text`, `image`,
    /// `audio`). Empty means the row said nothing, never "text only".
    pub input_modalities: Vec<String>,
    /// The provider's own input price, verbatim, unit included when the
    /// provider stated one. Text rather than a number: a price is a
    /// float and a float does not belong in a payload this city writes
    /// down, and the units providers quote in do not agree.
    pub input_price: Option<String>,
    /// The provider's own output price, verbatim. See `input_price`.
    pub output_price: Option<String>,
    /// The thinking levels this (Endpoint, model) offers, and the rung of
    /// `gateway::provider::thinking`'s ladder that said so.
    pub thinking: super::ThinkingOffer,
    /// This model's identity across providers: two rows are one model
    /// exactly when these are equal.
    pub canonical: String,
}
