// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which outside service a run's `web_search` reaches: the shape only.
//!
//! What `Default` points at, how `selected` is found, what a file may not
//! say and on which rungs it may say it are all `city`'s answers
//! (`crates/city/spec/ConfigLayers.lean` §8-4c, city D25), so the default
//! supplier's address is declared there once and has no copy here.
//!
//! Specified by `crates/kernel/spec/Config.lean` §8-22.

use serde::{Deserialize, Serialize};

use crate::event::record::ProviderAccount;
use crate::tool::ServerLabel;

/// One search service reached over MCP streamable HTTP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SearchSupplier {
    /// The supplier's name, and the Connector label `web_search` carries.
    pub id: ServerLabel,
    /// The MCP HTTP address, judged like an `[[mcp]]` url (city D26).
    pub url: String,
    /// The name of the remote tool in this supplier's `tools/list`.
    pub remote: String,
    /// The remote parameter `web_search`'s `query` is handed to.
    pub query_field: String,
    /// The remote parameter `objective` is handed to; `None` when this
    /// supplier takes no objective.
    pub objective_field: Option<String>,
    /// The remote parameter `num_results` is handed to; `None` when this
    /// supplier takes no count.
    pub count_field: Option<String>,
    /// In priority order. An anonymous account is the row without a
    /// reference; a key appears only as its vault reference.
    pub accounts: Vec<ProviderAccount>,
}

/// What a layer says about web search. Three arms and no `enabled`
/// switch, so each of the three intents has one spelling: the default
/// supplier, this supplier, none. `Default` is a stated value, not an
/// absence: a layer that states it covers a farther layer's `Custom`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SearchConfiguration {
    /// The supplier `city` declares as the default.
    #[default]
    Default,
    /// Exactly the listed supplier whose id is `selected`; never another.
    Custom {
        selected: ServerLabel,
        suppliers: Vec<SearchSupplier>,
    },
    /// No `web_search` under this layer.
    Off,
}
