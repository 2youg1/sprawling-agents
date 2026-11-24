// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, EventKind, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunSummary {
    pub run: RunId,
    pub who: String,
    pub frozen: bool,
    pub last_seq: Seq,
    pub last_kind: EventKind,

    pub addr: Option<Address>,

    pub started: Option<TimeMs>,

    pub completion: Option<String>,

    pub pr: Option<String>,

    pub ask: Option<String>,

    pub task: Option<String>,

    pub goal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_began: Option<Seq>,

    #[serde(default)]
    pub waiting: Option<Waiting>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Waiting {
    pub on: Address,
    pub until: TimeMs,
}
