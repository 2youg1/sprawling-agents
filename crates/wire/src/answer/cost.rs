// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's cost view: one authoritative total and the cuts of it a
//! page draws (`crates/wire/spec/Server.lean` §8-2).

use kernel::UsdMicros;
use serde::{Deserialize, Serialize};

/// The five cuts of one authoritative total. Four cuts sum to `total`;
/// `by_run` names the active runs and the few billed most, so it may sum
/// to less. Shares render against `total`, so a remainder stays visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CostAnswer {
    pub total: UsdMicros,
    pub by_run: Vec<(String, UsdMicros)>,
    pub by_actor: Vec<(String, UsdMicros)>,
    pub by_segment: Vec<(String, UsdMicros)>,
    pub by_tool: Vec<(String, UsdMicros)>,
    pub by_skill: Vec<(String, UsdMicros)>,
    /// The calls no provider priced, which `total` cannot show.
    pub unpriced: UnpricedCalls,
}

/// The model calls that came back with no authoritative amount, and the
/// tokens they used. A city whose provider never prices a call has a
/// zero `total` after any number of runs; this is what tells that city
/// apart from one where nothing ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UnpricedCalls {
    pub calls: u64,
    pub tokens: u64,
}
