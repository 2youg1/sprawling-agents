// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::{B3Hash, ServerLabel};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CatalogSubject {
    Skill { name: String },
    Server { label: ServerLabel },
    Tool { label: ServerLabel, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogDrift {
    pub subject: CatalogSubject,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub pinned: Option<B3Hash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub found: Option<B3Hash>,
}

impl CatalogDrift {

    #[must_use]
    pub fn between(
        subject: CatalogSubject,
        pinned: Option<B3Hash>,
        found: Option<B3Hash>,
    ) -> Option<CatalogDrift> {
        if pinned == found {
            return None;
        }
        Some(CatalogDrift {
            subject,
            pinned,
            found,
        })
    }
}
