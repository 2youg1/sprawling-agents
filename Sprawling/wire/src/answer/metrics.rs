// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's vital signs, counted once so a readout is one question.

use serde::{Deserialize, Serialize};

/// The city's vital signs: the counts a page would otherwise assemble
/// by asking four questions and adding up the answers.
///
/// Every number here is already proven by another view; this query
/// exists so that drawing one readout costs one question. It holds no
/// money - that is `CostView`'s, and one figure with two owners is how
/// two figures start disagreeing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MetricsAnswer {
    pub events: u64,
    pub runs_active: u64,
    pub runs_frozen: u64,
    pub buildings: u64,
    pub approvals_waiting: u64,
    pub signals_waiting: u64,
    /// Discarded and not yet restored.
    pub discards_outstanding: u64,
}
