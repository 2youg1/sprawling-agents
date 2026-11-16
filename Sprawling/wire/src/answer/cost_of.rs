// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{NodeId, RunId, UsdMicros};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CostOfAnswer {
    pub node: NodeId,

    pub spent: UsdMicros,

    pub runs: Vec<(RunId, UsdMicros)>,
}

pub const RUN_COSTS_MAX: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunCostsAnswer {

    pub asked: Vec<RunId>,

    pub runs: Vec<(RunId, UsdMicros)>,
}
