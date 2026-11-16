// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MetricsAnswer {
    pub events: u64,
    pub runs_active: u64,
    pub runs_frozen: u64,
    pub buildings: u64,
    pub approvals_waiting: u64,
    pub signals_waiting: u64,

    pub discards_outstanding: u64,
}
