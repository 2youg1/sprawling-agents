// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::Listing;
use kernel::{Ceiling, model::Window};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelFactsSummary {
    pub id: String,

    pub context_tokens: Option<Window>,

    pub max_output_tokens: Option<Ceiling>,

    pub input_modalities: Vec<String>,

    pub input_price: Option<String>,

    pub output_price: Option<String>,

    pub thinking: super::ThinkingOffer,

    pub generation: GenerationAnswer,

    pub canonical: String,

    pub listing: Listing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParameterAnswer {
    pub support: kernel::ParameterSupport,

    pub source: Option<kernel::ParameterSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GenerationAnswer {
    pub settings: kernel::GenerationSettings,

    pub output_tokens: Option<Ceiling>,
    pub temperature: ParameterAnswer,
    pub top_p: ParameterAnswer,
    pub top_k: ParameterAnswer,

    pub output_pointer: String,
}
