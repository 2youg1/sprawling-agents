// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a new building is laid out as, and the bytes each layout starts
//! it with.
//!
//! The words a template is named by live here and nowhere else: the
//! parser reads them back through `name`, so a template added to the
//! enum is one the control surface can ask for and one the refusal
//! lists, without a second table to keep in step.

use kernel::{Address, AxCode, AxError};

/// What a new building is and what it may do: one document, read by
/// the city and by every resident of the building. Instantiated at
/// compile time so that a moved or renamed template breaks the build
/// rather than a city.
const TEMPLATE_RULES: &str = include_str!("../../../../docs/templates/RULES.toml");
/// City Hall's rules, fixed rather than derived: what its two residents
/// may do serves every other building, so it is a property of the city.
const HALL_RULES: &str = include_str!("../../../../docs/templates/RULES-hall.toml");
/// The word every document template carries where a building's own
/// name goes. One spelling for the rules laid out here and for the
/// plan, memo and handoff `crate::spine_files` lays out beside them: a
/// second spelling would leave one of those documents addressed to
/// `<building name>` for the life of the building.
pub(crate) const NAME_PLACEHOLDER: &str = "<building name>";
const ORDINARY_LINE: &str = "confidential = false";
const CONFIDENTIAL_LINE: &str = "confidential = true";

/// What a new building is laid out as. Exhaustive: a template exists
/// because some kind of building needs different bytes on its first day,
/// and a kind nobody creates is an authority nobody reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingTemplate {
    /// An ordinary building: data may leave, any model may answer.
    Minimal,
    /// Data enters and does not leave, the model pool is local, and
    /// writes stop at this building's own subtree.
    Confidential,
    /// City Hall: raised with the city, writing Markdown documents and
    /// never a plan file, holding no project of its own.
    Hall,
}

impl BuildingTemplate {
    /// Every template, in the order a person is offered them.
    pub const ALL: [BuildingTemplate; 3] = [
        BuildingTemplate::Minimal,
        BuildingTemplate::Confidential,
        BuildingTemplate::Hall,
    ];

    /// Reads a template name as it arrived from the control surface.
    ///
    /// Read back through [`BuildingTemplate::name`] rather than against
    /// a second list of words: the refusal then spells exactly the set
    /// this version lays out, and a template added to the enum cannot
    /// be one the parser rejects.
    ///
    /// # Errors
    /// Refuses a name this version does not lay out, and says which ones
    /// it does: a caller that guessed needs the list, not a verdict.
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

    /// The word the control surface, the ledger and every document
    /// spell this template with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            BuildingTemplate::Minimal => "minimal",
            BuildingTemplate::Confidential => "confidential",
            // City Hall is the one building whose address is fixed by
            // the city rather than chosen by a person, and its template
            // is named after it, so the two read the same constant.
            BuildingTemplate::Hall => kernel::consts_policy::HALL_BUILDING,
        }
    }

    /// The document this template starts a building with.
    ///
    /// # Errors
    /// Refuses when the template no longer carries the line a
    /// confidential building differs by: producing an ordinary building
    /// from the confidential template is the one failure here that
    /// nobody would notice until data left.
    pub(super) fn rules(self, addr: &Address) -> Result<String, AxError> {
        let named = TEMPLATE_RULES.replace(NAME_PLACEHOLDER, addr.as_str());
        match self {
            BuildingTemplate::Minimal => Ok(named),
            // Fixed bytes, and the address is not substituted into them:
            // City Hall is one address in every city.
            BuildingTemplate::Hall => Ok(HALL_RULES.to_owned()),
            BuildingTemplate::Confidential => {
                if !named.contains(ORDINARY_LINE) {
                    return Err(AxError::failure(
                        AxCode::ConfigInvalid,
                        "lay out a confidential building",
                        format!("the template no longer carries `{ORDINARY_LINE}`"),
                    )
                    .with_recovery(
                        "restore that line in docs/templates/RULES.toml; the confidential \
                         template is the ordinary one with that value flipped",
                    ));
                }
                Ok(named.replace(ORDINARY_LINE, CONFIDENTIAL_LINE))
            }
        }
    }
}
