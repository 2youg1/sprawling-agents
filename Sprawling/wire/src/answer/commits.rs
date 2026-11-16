// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash, Effort, GitOid, RunId, Seq, SessionName, TimeMs, UsdMicros};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitAnswer {
    pub oid: GitOid,
    pub run: RunId,

    pub actor: Address,

    pub model: String,

    pub effort: Option<Effort>,

    pub seq: Seq,

    pub at: TimeMs,

    pub session: Option<SessionName>,

    pub lineage: Vec<RunId>,

    pub spent: UsdMicros,

    pub previous: Option<CommitAt>,

    pub parents: Option<Vec<GitOid>>,

    pub message: Option<String>,

    #[serde(default)]
    pub b3: Option<B3Hash>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitAt {
    pub oid: GitOid,
    pub seq: Seq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitsAnswer {
    pub building: Option<Address>,
    pub before: Option<Seq>,
    pub commits: Vec<CommitAnswer>,
    pub more: bool,
}
