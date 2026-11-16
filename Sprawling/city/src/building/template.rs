// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError};

const TEMPLATE_RULES: &str = include_str!("../../templates/RULES.toml");

const HALL_RULES: &str = include_str!("../../templates/RULES-hall.toml");

pub(crate) const NAME_PLACEHOLDER: &str = "<building name>";
const ORDINARY_LINE: &str = "confidential = false";
const CONFIDENTIAL_LINE: &str = "confidential = true";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingTemplate {

    Minimal,

    Confidential,

    Hall,
}

impl BuildingTemplate {

    pub const ALL: [BuildingTemplate; 3] = [
        BuildingTemplate::Minimal,
        BuildingTemplate::Confidential,
        BuildingTemplate::Hall,
    ];

    pub fn parse(name: &str) -> Result<BuildingTemplate, AxError> {
        let known = BuildingTemplate::ALL
            .into_iter()
            .find(|template| template.name() == name);
        known.ok_or_else(|| {
            let offered: Vec<String> = BuildingTemplate::ALL
                .into_iter()
                .map(|template| format!("`{}`", template.name()))
                .collect();
            AxError::failure(
                AxCode::InvalidArgs,
                "read a building template name",
                name.to_owned(),
            )
            .with_recovery(format!("this version lays out {}", offered.join(", ")))
        })
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            BuildingTemplate::Minimal => "minimal",
            BuildingTemplate::Confidential => "confidential",

            BuildingTemplate::Hall => kernel::consts_policy::HALL_BUILDING,
        }
    }

    pub(super) fn rules(self, addr: &Address) -> Result<String, AxError> {
        let named = TEMPLATE_RULES.replace(NAME_PLACEHOLDER, addr.as_str());
        match self {
            BuildingTemplate::Minimal => Ok(named),

            BuildingTemplate::Hall => Ok(HALL_RULES.to_owned()),
            BuildingTemplate::Confidential => {
                if !named.contains(ORDINARY_LINE) {
                    return Err(AxError::failure(
                        AxCode::ConfigInvalid,
                        "lay out a confidential building",
                        format!("the template no longer carries `{ORDINARY_LINE}`"),
                    )
                    .with_recovery(
                        "restore that line in Sprawling/city/templates/RULES.toml; the confidential \
                         template is the ordinary one with that value flipped",
                    ));
                }
                Ok(named.replace(ORDINARY_LINE, CONFIDENTIAL_LINE))
            }
        }
    }
}
