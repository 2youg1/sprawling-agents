// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::B3Hash;
use serde::{Deserialize, Serialize};

use crate::command::GovernedDocument;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum IdentityAnswer {
    Stated(StatedIdentity),
    Unreadable {
        document: GovernedDocument,

        line: u32,
        why: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StatedIdentity {
    pub user_id: Option<String>,

    pub imported_from: Option<String>,

    pub about: String,

    pub mayor: Option<String>,

    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub version: B3Hash,
    pub preferences_text: String,
    pub mayor_text: String,
}
