// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use documents::{Slice, Span};
use kernel::{Address, B3Hash, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalsAnswer {
    pub doc: Address,

    pub version: Option<B3Hash>,

    pub open: Vec<ProposalCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalCard {
    pub id: B3Hash,
    pub run: RunId,
    pub baseline: B3Hash,
    pub span: Span,
    pub slices: Vec<Slice>,

    #[serde(default)]
    pub offered: Option<Seq>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OpenProposalsAnswer {
    pub open: Vec<OfferedCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OfferedCard {
    pub doc: Address,
    pub id: B3Hash,

    pub at: Option<TimeMs>,
}
