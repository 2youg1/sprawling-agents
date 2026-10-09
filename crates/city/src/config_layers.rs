// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three configuration files a run is governed by, and where they
//! live.
//!
//! One file name, three locations: the layer is the position, so a file
//! placed at the wrong layer is a mistake you can see in the directory
//! tree rather than one you have to read character by character. The
//! city's own layer lives inside the reserved subtree, which is outside
//! every write domain — that is what makes "an agent cannot edit its own
//! configuration" a judgment instead of an expectation.
//!
//! Which layer wins and what an unstated value falls back to are
//! `kernel::config`'s answers, not this module's; which layers there
//! are and what each states is [`ladder`]'s. This module answers what
//! one file says.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4.

use std::path::{Path, PathBuf};

use kernel::config::{SearchConfiguration, StatedConfig};
use kernel::{
    Address, AxError, B3Hash, ClockStampGranularity, Effort, FrozenConfig, KeepWarm, McpServer,
    SandboxLimits, SecondThreshold, ServerLabel,
};
use serde::Deserialize;

use cache::CacheSection;
use clock::ClockSection;
use context::ContextSection;
mod agents;
mod cache;
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

pub use agents::{AgentRow, AgentRowSource, agent_rows};
pub use cache::keep_warm;
pub use city_layer::{CitySetting, write_city_setting};
pub use ladder::Layer;
pub use remote::{HostPermanence, RemoteRoute, remote_route};
pub use resident::settled_harness;
pub use search::{city_search, default_search_supplier};
pub use search::{search_supplier, settled_search, write_search};
pub(crate) use session::forget as forget_session;
pub use session::{freeze_naming, own_layer, write_session};
pub use settled::{settled_effort, settled_second};
pub(crate) use shelves::SHELVES_KEY;
pub use shelves::city_shelves;
pub use write::{write_mcp, write_sandbox, write_second_threshold};

use ladder::Ladder;
// The refusal shapes every reader in this module answers with.
use refuse::{Confined, refuse, unreadable};

/// Where a layer's file lives for a run at `addr`.
///
/// # Errors
/// Propagates the reserved-subtree refusal a layer reports for an
/// address with no building, which therefore has no building layer.
pub fn path(city_root: &Path, addr: &Address, layer: Layer) -> Result<PathBuf, AxError> {
    layer.file(city_root, addr)
}

/// What one layer declares. An absent file declares nothing, which is
/// how most layers stay: a value is stated where somebody meant to
/// depart from the default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigLayer {
    /// The model a scope froze, written once when a session opens.
    model: Option<String>,
    /// The harness this scope names as the resident of its rooms, as written.
    harness: Option<String>,
    effort: Option<Effort>,
    sandbox: Option<SandboxLimits>,
    mcp: Option<Vec<McpServer>>,
    /// Which supplier `web_search` reaches (§8-4c); refused on a room's rung.
    search: Option<SearchConfiguration>,
    /// The second context-reminder rung this layer states.
    second_threshold: Option<SecondThreshold>,
    /// How often this layer's results carry the clock line.
    clock_stamp: Option<ClockStampGranularity>,
    keep_warm: Option<KeepWarm>,
    shelves: Option<Vec<String>>,
    /// The identity version a session froze at this address
    /// (`crates/city/spec/Identity.lean` §8-33): read at the room's own layer only.
    naming: Option<B3Hash>,
    /// The route the city's own layer chooses for the remote door
    /// (`crates/city/spec/ConfigLayers/Remote.lean` §8-39); refused on every other rung.
    remote: Option<RemoteRoute>,
    /// The ACP agents the person added (§8-4f); refused on every other rung.
    agents: Option<Vec<AgentRow>>,
}

impl ConfigLayer {
    /// Reads one layer's text.
    ///
    /// # Errors
    /// Refuses a file that does not parse, and a key this version does
    /// not read. Ignoring an unrecognised key produces the one state
    /// nobody can diagnose: the setting is written, and nothing happens.
    pub fn parse(text: &str) -> Result<ConfigLayer, AxError> {
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
                // Refused here rather than where a child would be
                // started: a name that reached a process cannot be
                // taken back, and the person holding the refusal is
                // the one editing this file.
                let mut env_passthrough = Vec::new();
                for raw in &section.env_passthrough {
                    let name = kernel::EnvVarName::parse(raw)
                        .map_err(|err| refuse(format!("{raw}: {}", err.recovery())))?;
                    env_passthrough.push(name);
                }
                // Which connector may reach what nothing here can
                // undo, named server by server in the one file a
                // person edits (`kernel::SandboxLimits::trusted`).
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
        let mcp = file.mcp.map(mcp::servers).transpose()?;
        let search = file.search.map(search::stated).transpose()?;
        let model = resident::stated_name(file.model.name, resident::MODEL_NAME_KEY)?;
        let harness = file.resident.and_then(|section| section.harness);
        let harness = resident::stated_name(harness, resident::HARNESS_KEY)?;
        resident::one_resident(model.as_deref(), harness.as_deref())?;
        let second_threshold = file
            .context
            .map(|section| SecondThreshold::parse(section.second_threshold))
            .transpose()?;
        Ok(ConfigLayer {
            model,
            harness,
            effort: file.model.effort,
            sandbox,
            mcp,
            search,
            second_threshold,
            clock_stamp: file.clock.map(|section| section.stamp),
            keep_warm: file.cache.map(|section| section.keep_warm),
            // The paths are kept as written: turning `~` into a
            // directory needs this person's home, which is not this
            // module's to read, and a value stored half-resolved would
            // be a second spelling of the same shelf.
            shelves: file.skills.map(|section| section.shelves),
            naming: file.identity.map(|section| section.version),
            remote: file.remote.map(remote::stated).transpose()?,
            agents: file.agent.map(agents::rows).transpose()?,
        })
    }

    /// The model this scope froze, as written.
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    #[must_use]
    pub fn effort(&self) -> Option<Effort> {
        self.effort
    }

    /// The second context-reminder rung this layer states. Taken
    /// (`SecondThreshold`) rather than raw: what is spelled in a file
    /// and refused sits before this value exists.
    #[must_use]
    pub fn second_threshold(&self) -> Option<SecondThreshold> {
        self.second_threshold
    }

    /// How often this layer asks for the clock line (`crates/city/spec/ConfigLayers.lean` §8-31).
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
    pub fn mcp(&self) -> Option<&[McpServer]> {
        self.mcp.as_deref()
    }

    /// The identity version a session at this address froze, when it
    /// froze one.
    #[must_use]
    pub fn naming(&self) -> Option<B3Hash> {
        self.naming
    }

    /// The directories this layer mounts read-only beside the city's own
    /// shelves, as written. Empty when the layer has no `[skills]` table.
    #[must_use]
    pub fn shelves(&self) -> Option<&[String]> {
        self.shelves.as_deref()
    }

    /// The route this layer chooses for the remote door, as written.
    #[must_use]
    pub fn remote(&self) -> Option<&RemoteRoute> {
        self.remote.as_ref()
    }

    /// The ACP agents this layer adds, as written.
    #[must_use]
    pub fn agents(&self) -> Option<&[AgentRow]> {
        self.agents.as_deref()
    }

    /// Every table this layer states that only farther rungs may state,
    /// which the ladder refuses on a rung nearer than the table allows.
    fn confined(&self) -> Vec<Confined> {
        [
            self.shelves.as_ref().map(|_| Confined::Shelves),
            self.remote.as_ref().map(|_| Confined::Remote),
            self.search.as_ref().map(|_| Confined::Search),
            self.agents.as_ref().map(|_| Confined::Agents),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// Resolves the ladder into the snapshot a run is frozen with.
///
/// Concern by concern rather than layer by layer: the ladder knows
/// which layers exist, so this reads as the list of what a run is
/// governed by, and a layer added later is picked up by all of it at
/// once.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse. A missing file is not an error: it is the
/// ordinary case.
pub fn load(city_root: &Path, addr: &Address) -> Result<FrozenConfig, AxError> {
    let ladder = Ladder::read(city_root, addr)?;
    Ok(kernel::config::freeze(&StatedConfig {
        clock_stamp: ladder.resolve(ConfigLayer::clock_stamp),
        // No layer states zones: a stamp is written in UTC, and the key
        // is refused where it is written (`crates/city/spec/ConfigLayers.lean` §8-31).
        clock_zones: kernel::LayeredValue::default(),
        effort: ladder.resolve(ConfigLayer::effort),
        sandbox: ladder.resolve(|layer| layer.sandbox().cloned()),
        mcp: ladder.resolve(|layer| layer.mcp().map(<[McpServer]>::to_vec)),
        second_threshold: ladder.resolve(|layer| layer.second_threshold),
        search: ladder.resolve(|layer| layer.search().cloned()),
    }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigFile {
    #[serde(default)]
    model: ModelSection,
    #[serde(default)]
    sandbox: Option<SandboxSection>,
    #[serde(default)]
    mcp: Option<Vec<mcp::McpSection>>,
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
}

/// The `[identity]` table: the version of the names a session froze,
/// written by the session's first run and taken out by `/new`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentitySection {
    version: B3Hash,
}

/// The `[skills]` table: the directories outside the city this city
/// mounts read-only beside its own shelves.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillsSection {
    #[serde(default)]
    shelves: Vec<String>,
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
    /// Read as text and judged by `kernel::Interpreter::parse`, so the
    /// refusal names both spellings rather than serde's variant list.
    #[serde(default)]
    interpreter: Option<String>,
    #[serde(default)]
    fuel: Option<u64>,
    #[serde(default)]
    mounts: Vec<String>,
    #[serde(default)]
    env_passthrough: Vec<String>,
    /// Empty unless a person wrote a label here: an effect nothing can
    /// take back is allowed one server at a time.
    #[serde(default)]
    trusted: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelSection {
    /// What a session calls, kept for every run after its first.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    effort: Option<Effort>,
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
