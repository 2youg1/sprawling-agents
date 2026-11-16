// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ProviderFailureKind {

    Exchange,

    Cut,

    Silence,

    Refused { status: u32 },

    Quota { status: u32 },

    Overflow { status: u32 },

    Unreadable,

    Reported,

    Unbuilt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AccountDisposition {

    #[default]
    Keep,

    Advance,
}

impl AccountDisposition {

    pub(crate) fn is_keep(&self) -> bool {
        *self == AccountDisposition::Keep
    }
}
