// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::event::record::ProviderAccount;
use crate::tool::ServerLabel;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SearchSupplier {
    pub id: ServerLabel,

    pub url: String,

    pub remote: String,

    pub query_field: String,

    pub objective_field: Option<String>,

    pub count_field: Option<String>,

    pub accounts: Vec<ProviderAccount>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SearchConfiguration {
    #[default]
    Default,

    Custom {
        selected: ServerLabel,
        suppliers: Vec<SearchSupplier>,
    },

    Off,
}
