// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, Effort, Origin, Seq, TimeMs};
use serde::{Deserialize, Serialize};

use crate::command::Carry;

pub const SESSIONS_MAX: usize = 64;

pub const SESSION_PREVIEW_MAX: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionsAnswer {
    pub room: Address,
    pub sessions: Vec<SessionLine>,
    pub earlier: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionRef {
    pub room: Address,
    pub began: Seq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionLine {
    pub began: Seq,
    pub start: SessionStart,

    pub runs: u64,

    pub last: Seq,
    pub at: TimeMs,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub model: Option<String>,

    #[serde(default)]
    pub effort: Option<Effort>,

    #[serde(default)]
    pub workspace: Option<String>,

    #[serde(default)]
    pub preview: Option<String>,

    #[serde(default)]
    pub parent: Option<SessionRef>,

    #[serde(default)]
    pub branched_from: Option<Origin>,

    #[serde(default)]
    pub tail: Option<Origin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SessionStart {
    Dispatched {
        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        by: Option<kernel::event::Who>,
    },

    Opened {
        carry: Carry,
        from: Option<Origin>,
    },
}
