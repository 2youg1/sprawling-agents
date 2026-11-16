// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::budget::UsdMicros;
use crate::error::AxError;
use crate::model::ModelUsage;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixHeld {
    pub waited_us: u64,

    pub ended: PrewarmEnd,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent: Option<PrewarmSent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PrewarmEnd {
    Answered,

    Refused,

    TimedOut,

    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum PrewarmSent {
    Refused {
        refused: AxError,
    },

    Answered {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(
            feature = "schema",
            schemars(with = "Option<std::collections::BTreeMap<String, u64>>")
        )]
        usage: Option<ModelUsage>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        billed_usd_micros: Option<UsdMicros>,
    },
}
