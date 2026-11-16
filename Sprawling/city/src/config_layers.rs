// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::config::{SearchConfiguration, StatedConfig};
use kernel::{
    Address, AxError, B3Hash, ClockStampGranularity, Effort, FrozenConfig, KeepWarm, McpLayer,
    McpReference, SandboxLimits, SecondThreshold, ServerLabel,
};
use serde::Deserialize;

use cache::CacheSection;
use clock::ClockSection;
use context::ContextSection;
mod agents;
mod cache;
mod catalog;
mod catalog_session;
mod city_layer;
mod clock;
mod context;
mod ladder;
mod mcp;
mod refuse;
mod remote;
mod resident;
mod search;
mod session;
mod settled;
mod shelves;
mod write;

pub use agents::{AgentRow, AgentRowSource, agent_rows, seat_agent, write_agent};
pub use cache::keep_warm;
pub use catalog::{check as check_catalog, write_catalog};
pub(crate) use catalog_session::forget as forget_catalog_session;
pub use catalog_session::{
    SessionCatalog, catalog_for_session, migrate_session_mcp_reference, migrate_session_skill_pin,
    previous_sessions,
};
pub use city_layer::{CitySetting, write_city_setting};
pub use ladder::Layer;
pub use remote::{HostPermanence, RemoteRoute, remote_route};
pub use resident::settled_harness;
pub use search::{city_search, default_search_supplier};
pub use search::{search_supplier, settled_search, write_search};
pub(crate) use session::forget as forget_session;
pub use session::{SessionShape, freeze_naming, own_layer, write_session};
pub use settled::{settled_effort, settled_second};
pub(crate) use shelves::SHELVES_KEY;
pub use shelves::{check_skill_shelves, city_shelves, stated_skill_shelves};
pub use write::{migrate_mcp, migrate_mcp_reference, migrate_skill_pin};
pub use write::{write_mcp, write_sandbox, write_second_threshold};
pub use write::{write_mcp_references, write_skill_pins};

use ladder::Ladder;

use refuse::{Confined, refuse, unreadable};

pub fn path(city_root: &Path, addr: &Address, layer: Layer) -> Result<PathBuf, AxError> {
    layer.file(city_root, addr)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigLayer {
    model: Option<String>,
    endpoint: Option<String>,

    harness: Option<String>,
    effort: Option<Effort>,

    generation: Option<kernel::FrozenGeneration>,
    sandbox: Option<SandboxLimits>,
    mcp: Option<McpLayer>,

    search: Option<SearchConfiguration>,

    second_threshold: Option<SecondThreshold>,

    clock_stamp: Option<ClockStampGranularity>,
    keep_warm: Option<KeepWarm>,
    shelves: Option<Vec<String>>,

    skill_pins: std::collections::BTreeMap<String, B3Hash>,

    naming: Option<B3Hash>,

    remote: Option<RemoteRoute>,

    agents: Option<Vec<AgentRow>>,

    catalog: Option<kernel::config::ExposureTable>,
}

impl ConfigLayer {
    pub fn parse(text: &str) -> Result<ConfigLayer, AxError> {
        crate::format::admit_format(crate::format::FormatFile::LayerConfig, text)?;
        let file: ConfigFile = toml::from_str(text).map_err(|err| unreadable(text, &err))?;
        let sandbox = match file.sandbox {
            None => None,
            Some(section) => {
                let mut mounts = Vec::new();
                for raw in &section.mounts {
                    let mount = Address::parse(raw)?;
                    if mount.is_reserved() {
                        return Err(refuse(format!(
                            "{raw}: the city's own subtree is not mountable"
                        )));
                    }
                    mounts.push(mount);
                }

                let mut env_passthrough = Vec::new();
                for raw in &section.env_passthrough {
                    let name = kernel::EnvVarName::parse(raw)
                        .map_err(|err| refuse(format!("{raw}: {}", err.recovery())))?;
                    env_passthrough.push(name);
                }

                let mut trusted = Vec::new();
                for raw in &section.trusted {
                    let label = ServerLabel::parse(raw)
                        .map_err(|err| refuse(format!("{raw}: {}", err.recovery())))?;
                    trusted.push(label);
                }
                let interpreter = section
                    .interpreter
                    .as_deref()
                    .map_or(Ok(kernel::Interpreter::System), kernel::Interpreter::parse)?;
                let limits = SandboxLimits {
                    arm: section.arm,
                    container: section.container,
                    shell: section.shell,
                    interpreter,
                    fuel: section
                        .fuel
                        .unwrap_or(kernel::consts_policy::SANDBOX_FUEL_DEFAULT),
                    mounts,
                    env_passthrough,
                    trusted,
                };
                limits.selected_arm()?;
                Some(limits)
            }
        };
        let definitions = file.mcp.map(mcp::servers).transpose()?;
        let references = file.mcp_ref.map(mcp::references).transpose()?;
        let mcp = if definitions.is_some() || references.is_some() {
            Some(McpLayer {
                definitions,
                references,
            })
        } else {
            None
        };
        let search = file.search.map(search::stated).transpose()?;
        let model = resident::stated_name(file.model.name, resident::MODEL_NAME_KEY)?;
        let generation = (file.model.temperature.is_some()
            || file.model.top_p.is_some()
            || file.model.top_k.is_some()
            || file.model.max_tokens.is_some()
            || !file.model.overrides.is_empty())
        .then_some(kernel::FrozenGeneration {
            settings: kernel::GenerationSettings {
                temperature: file.model.temperature,
                top_p: file.model.top_p,
                top_k: file.model.top_k,
            },
            max_tokens: file.model.max_tokens,
            overrides: file.model.overrides,
        });
        let harness = file.resident.and_then(|section| section.harness);
        let harness = resident::stated_name(harness, resident::HARNESS_KEY)?;
        resident::one_resident(model.as_deref(), harness.as_deref())?;
        let second_threshold = file
            .context
            .map(|section| SecondThreshold::parse(section.second_threshold))
            .transpose()?;
        Ok(ConfigLayer {
            endpoint: file.model.endpoint,
            model,
            harness,
            effort: file.model.effort,
            generation,
            sandbox,
            mcp,
            search,
            second_threshold,
            clock_stamp: file.clock.map(|section| section.stamp),
            keep_warm: file.cache.map(|section| section.keep_warm),

            shelves: file
                .skills
                .as_ref()
                .and_then(|section| section.shelves.clone()),
            skill_pins: file.skills.map(|section| section.pins).unwrap_or_default(),
            naming: file.identity.map(|section| section.version),
            remote: file.remote.map(remote::stated).transpose()?,
            agents: file.agent.map(agents::rows).transpose()?,
            catalog: file.catalog.map(catalog::stated).transpose()?,
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    #[must_use]
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    #[must_use]
    pub fn effort(&self) -> Option<Effort> {
        self.effort
    }

    #[must_use]
    pub fn generation(&self) -> Option<&kernel::FrozenGeneration> {
        self.generation.as_ref()
    }

    #[must_use]
    pub fn second_threshold(&self) -> Option<SecondThreshold> {
        self.second_threshold
    }

    #[must_use]
    pub fn clock_stamp(&self) -> Option<ClockStampGranularity> {
        self.clock_stamp
    }

    #[must_use]
    pub fn keep_warm(&self) -> Option<KeepWarm> {
        self.keep_warm
    }

    #[must_use]
    pub fn sandbox(&self) -> Option<&SandboxLimits> {
        self.sandbox.as_ref()
    }

    #[must_use]
    pub fn mcp(&self) -> Option<&McpLayer> {
        self.mcp.as_ref()
    }

    #[must_use]
    pub fn skill_pins(&self) -> &std::collections::BTreeMap<String, B3Hash> {
        &self.skill_pins
    }

    #[must_use]
    pub fn naming(&self) -> Option<B3Hash> {
        self.naming
    }

    #[must_use]
    pub fn shelves(&self) -> Option<&[String]> {
        self.shelves.as_deref()
    }

    #[must_use]
    pub fn remote(&self) -> Option<&RemoteRoute> {
        self.remote.as_ref()
    }

    #[must_use]
    pub fn agents(&self) -> Option<&[AgentRow]> {
        self.agents.as_deref()
    }

    fn confined(&self) -> Vec<Confined> {
        [
            self.shelves.as_ref().map(|_| Confined::Shelves),
            self.mcp
                .as_ref()
                .and_then(|mcp| mcp.references.as_ref())
                .map(|_| Confined::McpReferences),
            (!self.skill_pins.is_empty()).then_some(Confined::SkillPins),
            self.remote.as_ref().map(|_| Confined::Remote),
            self.search.as_ref().map(|_| Confined::Search),
            self.agents.as_ref().map(|_| Confined::Agents),
            self.catalog.as_ref().map(|_| Confined::Catalog),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

pub fn check_mcp_references(
    city_root: &Path,
    building: &Address,
    references: &[McpReference],
) -> Result<(), AxError> {
    let ladder = Ladder::read(city_root, building)?;
    let mut mcp = ladder.resolve(|layer| layer.mcp().cloned());
    let mut layer = mcp.building.take().unwrap_or_default();
    layer.references = Some(references.to_vec());
    mcp.building = Some(layer);
    kernel::config::freeze(&StatedConfig {
        mcp,
        ..StatedConfig::default()
    })
    .map(|_| ())
}

pub fn load(city_root: &Path, addr: &Address) -> Result<FrozenConfig, AxError> {
    let ladder = Ladder::read(city_root, addr)?;
    kernel::config::freeze(&StatedConfig {
        clock_stamp: ladder.resolve(ConfigLayer::clock_stamp),

        clock_zones: kernel::LayeredValue::default(),
        effort: ladder.resolve(ConfigLayer::effort),
        sandbox: ladder.resolve(|layer| layer.sandbox().cloned()),
        mcp: ladder.resolve(|layer| layer.mcp().cloned()),
        skill_pins: ladder
            .resolve(|layer| (!layer.skill_pins().is_empty()).then(|| layer.skill_pins().clone())),
        second_threshold: ladder.resolve(|layer| layer.second_threshold),
        search: ladder.resolve(|layer| layer.search().cloned()),
        catalog: ladder.resolve(|layer| layer.catalog().cloned()),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigFile {
    #[serde(default)]
    #[expect(dead_code, reason = "judged on the text before this struct is read")]
    format: Option<toml::Value>,
    #[serde(default)]
    model: ModelSection,
    #[serde(default)]
    sandbox: Option<SandboxSection>,
    #[serde(default)]
    mcp: Option<Vec<mcp::McpSection>>,
    #[serde(default)]
    mcp_ref: Option<Vec<mcp::McpReferenceSection>>,
    #[serde(default)]
    search: Option<search::SearchSection>,
    #[serde(default)]
    context: Option<ContextSection>,
    #[serde(default)]
    clock: Option<ClockSection>,
    #[serde(default)]
    cache: Option<CacheSection>,
    #[serde(default)]
    skills: Option<SkillsSection>,
    #[serde(default)]
    resident: Option<resident::ResidentSection>,
    #[serde(default)]
    identity: Option<IdentitySection>,
    #[serde(default)]
    remote: Option<RemoteRoute>,
    #[serde(default)]
    agent: Option<Vec<agents::AgentSection>>,
    #[serde(default)]
    catalog: Option<catalog::CatalogSection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentitySection {
    version: B3Hash,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillsSection {
    #[serde(default)]
    shelves: Option<Vec<String>>,
    #[serde(default)]
    pins: std::collections::BTreeMap<String, B3Hash>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SandboxSection {
    #[serde(default)]
    arm: Option<kernel::SandboxArm>,
    #[serde(default)]
    container: Option<kernel::ContainerLimits>,
    #[serde(default)]
    shell: bool,

    #[serde(default)]
    interpreter: Option<String>,
    #[serde(default)]
    fuel: Option<u64>,
    #[serde(default)]
    mounts: Vec<String>,
    #[serde(default)]
    env_passthrough: Vec<String>,

    #[serde(default)]
    trusted: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelSection {
    #[serde(default)]
    endpoint: Option<String>,

    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    effort: Option<Effort>,
    #[serde(default)]
    temperature: Option<String>,
    #[serde(default)]
    top_p: Option<String>,
    #[serde(default)]
    top_k: Option<String>,

    #[serde(default)]
    max_tokens: Option<kernel::Ceiling>,
    #[serde(default)]
    overrides: Vec<(String, String)>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
