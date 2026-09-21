// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an endpoint said about one model it serves.

use kernel::{Ceiling, model::Window};
use serde::{Deserialize, Serialize};

/// One model an endpoint serves, with what the endpoint said about it.
///
/// **An endpoint's list used to arrive as bare ids.** Everything the
/// provider stated beside each id — the window, the ceiling, what the
/// model accepts, what it costs — was read at attach, used once to
/// settle a ceiling, and then dropped, so a person choosing a model saw
/// a name and nothing to choose by. This is that statement, carried to
/// the page that shows the list.
///
/// **Every field is what the upstream said, not what this city
/// concluded.** Absence means the row said nothing; it never means
/// zero, and it is never filled in from the preset table here. The
/// ladder that picks a figure — the person's own entry, then the
/// upstream's statement, then the preset table, then the policy default
/// — runs where the call is made, and a summary that had already
/// climbed it would be a second answer to which figure won.
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
}
