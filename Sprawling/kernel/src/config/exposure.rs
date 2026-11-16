// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Exposure {
    Core,
    Dormant,
    Off,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ExposureTable {
    #[serde(default)]
    pub tools: BTreeMap<String, Exposure>,
    #[serde(default)]
    pub namespaces: BTreeMap<String, Exposure>,

    #[serde(default)]
    pub skills: BTreeMap<String, Exposure>,
}

impl ExposureTable {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.namespaces.is_empty() && self.skills.is_empty()
    }

    fn said(&self, tool: &str, namespace: &str) -> Option<Exposure> {
        self.tools
            .get(tool)
            .or_else(|| self.namespaces.get(namespace))
            .copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExposureFrom {
    Fixed,

    Default,

    City,

    Building,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogExposure {
    #[serde(default)]
    pub city: ExposureTable,
    #[serde(default)]
    pub building: ExposureTable,
}

impl CatalogExposure {
    #[must_use]
    pub fn stated(&self, tool: &str, namespace: &str) -> Option<(Exposure, ExposureFrom)> {
        self.building
            .said(tool, namespace)
            .map(|exposure| (exposure, ExposureFrom::Building))
            .or_else(|| {
                self.city
                    .said(tool, namespace)
                    .map(|exposure| (exposure, ExposureFrom::City))
            })
    }

    #[must_use]
    pub fn stated_skill(&self, skill: &str) -> Option<(Exposure, ExposureFrom)> {
        self.building
            .skills
            .get(skill)
            .map(|exposure| (*exposure, ExposureFrom::Building))
            .or_else(|| {
                self.city
                    .skills
                    .get(skill)
                    .map(|exposure| (*exposure, ExposureFrom::City))
            })
    }
}
