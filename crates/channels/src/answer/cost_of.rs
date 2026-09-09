// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one plan node has cost so far.
//!
//! **Absolute money and no share.** A share needs a denominator this
//! answer does not carry - the city's own total is `CostAnswer`'s - and
//! a percentage with no denominator is exactly what `UnplannedProgress`
//! refuses to spell. A reader who wants a share holds both answers and
//! divides.
//!
//! A node nobody ever claimed answers zero with an empty list rather
//! than `Unavailable`: "no run has held this node" is a true answer,
//! while "I could not look" is what `Unavailable` means.

use kernel::{NodeId, RunId, UsdMicros};
use serde::{Deserialize, Serialize};

/// What one plan node cost, and which runs spent it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CostOfAnswer {
    pub node: NodeId,
    /// The sum of `runs`, so a reader draws the figure without adding
    /// the rows up and getting a different number.
    pub spent: UsdMicros,
    /// One entry per run that claimed this node, in run order.
    pub runs: Vec<(RunId, UsdMicros)>,
}
