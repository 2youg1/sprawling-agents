// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{LayeredValue, McpServer};
use crate::{AxCode, AxError, B3Hash, ServerLabel};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpDefinition {
    pub server: McpServer,

    #[serde(default)]
    pub conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpReference {
    pub label: ServerLabel,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub definition: Option<B3Hash>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<BTreeMap<String, B3Hash>>,
}

impl McpReference {

    #[must_use]
    pub fn legacy(label: ServerLabel) -> McpReference {
        McpReference {
            label,
            definition: None,
            tools: None,
        }
    }

    #[must_use]
    pub fn is_legacy(&self) -> bool {
        self.definition.is_none() || self.tools.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpLayer {
    pub definitions: Option<Vec<McpDefinition>>,
    pub references: Option<Vec<McpReference>>,
}

impl McpLayer {
    fn selection(&self) -> Option<Vec<McpReference>> {
        self.references.clone().or_else(|| {
            self.definitions.as_ref().map(|definitions| {
                definitions
                    .iter()
                    .map(|definition| McpReference::legacy(definition.server.label.clone()))
                    .collect()
            })
        })
    }
}

pub fn definition_digest(server: &McpServer) -> Result<B3Hash, AxError> {
    serde_json::to_vec(server)
        .map(|bytes| B3Hash::digest(&bytes))
        .map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "pin an mcp server definition",
                server.label.as_str(),
            )
            .with_recovery(format!(
                "make the server definition serialisable as JSON: {err}"
            ))
        })
}

pub(super) fn resolve(
    stated: &LayeredValue<McpLayer>,
) -> Result<(Vec<McpServer>, Vec<McpReference>), AxError> {
    let layers = layers(stated);
    check_duplicates(&layers)?;
    let Some((scope, selected)) = layers
        .iter()
        .rev()
        .find_map(|(scope, layer)| layer.selection().map(|selected| (*scope, selected)))
    else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut seen = BTreeMap::new();
    let mut servers = Vec::with_capacity(selected.len());
    let mut references = Vec::with_capacity(selected.len());
    for reference in selected {
        let label = reference.label.as_str().to_owned();
        if seen.insert(label.clone(), ()).is_some() {
            return Err(refusal(&label, "the selection names this label twice"));
        }
        let definition = layers
            .iter()
            .filter(|(rungs, _)| *rungs <= scope)
            .flat_map(|(_, layer)| layer.definitions.iter().flatten())
            .rfind(|definition| definition.server.label == reference.label)
            .ok_or_else(|| refusal(&label, "no definition with this label exists"))?;
        servers.push(definition.server.clone());
        references.push(reference);
    }
    servers.sort_by(|left, right| left.label.as_str().cmp(right.label.as_str()));
    references.sort_by(|left, right| left.label.as_str().cmp(right.label.as_str()));
    Ok((servers, references))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rung {
    City,
    Building,
    Resident,
}

fn layers(stated: &LayeredValue<McpLayer>) -> Vec<(Rung, &McpLayer)> {
    [
        (Rung::City, stated.city.as_ref()),
        (Rung::Building, stated.building.as_ref()),
        (Rung::Resident, stated.resident.as_ref()),
    ]
    .into_iter()
    .filter_map(|(rung, layer)| layer.map(|layer| (rung, layer)))
    .collect()
}

fn check_duplicates(layers: &[(Rung, &McpLayer)]) -> Result<(), AxError> {
    let definitions = layers
        .iter()
        .flat_map(|(_, layer)| layer.definitions.iter().flatten())
        .collect::<Vec<_>>();
    for (index, farther) in definitions.iter().enumerate() {
        for nearer in definitions.iter().skip(index.saturating_add(1)) {
            if farther.server.label != nearer.server.label
                || farther.server == nearer.server
                || nearer.conflict
            {
                continue;
            }
            return Err(refusal(
                nearer.server.label.as_str(),
                "different definitions use one label at two scopes; keep only one, or mark the nearer definition `conflict = true`",
            ));
        }
    }
    Ok(())
}

fn refusal(label: &str, violation: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "resolve connector references",
        format!("{label}: {violation}"),
    )
    .with_recovery(
        "define the server once, reference it by label, and mark a deliberate building-local conflict explicitly",
    )
}
