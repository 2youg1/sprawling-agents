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
//! `kernel::config`'s answers, not this module's. This module answers
//! only which three files to read.

use std::path::{Path, PathBuf};

use kernel::{
    Address, AxCode, AxError, Effort, FrozenConfig, LayeredValue, McpServer, McpTransport,
    RESERVED_PREFIX, SandboxLimits, ServerLabel,
};
use serde::Deserialize;

use crate::building::Building;

mod write;

pub use write::{write_effort, write_mcp, write_sandbox};

/// The configuration file at every layer.
pub const CONFIG_FILE: &str = "CONFIG.toml";

/// One rung of the City -> Building -> Resident ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    City,
    Building,
    Resident,
}

/// Where a layer's file lives for a run at `addr`.
///
/// # Errors
/// Propagates the reserved-subtree refusal from [`Building::of`]: an
/// address with no building has no building layer to read.
pub fn path(city_root: &Path, addr: &Address, layer: Layer) -> Result<PathBuf, AxError> {
    // The scope this layer speaks for, then that scope's own reserved
    // subtree. One expression for three layers: the city's file is this
    // rule at the root scope rather than a case of its own, and a layer
    // added later cannot land somewhere its own runs may write.
    let scope = match layer {
        Layer::City => city_root.to_path_buf(),
        Layer::Building => Building::of(addr)?.root(city_root),
        Layer::Resident => {
            let mut path = city_root.to_path_buf();
            for segment in addr.as_str().split('/') {
                path.push(segment);
            }
            path
        }
    };
    Ok(scope.join(RESERVED_PREFIX).join(CONFIG_FILE))
}

/// What one layer declares. An absent file declares nothing, which is
/// how most layers stay: a value is stated where somebody meant to
/// depart from the default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigLayer {
    effort: Option<Effort>,
    sandbox: Option<SandboxLimits>,
    mcp: Option<Vec<McpServer>>,
}

impl ConfigLayer {
    /// Reads one layer's text.
    ///
    /// # Errors
    /// Refuses a file that does not parse, and a key this version does
    /// not read. Ignoring an unrecognised key produces the one state
    /// nobody can diagnose: the setting is written, and nothing happens.
    pub fn parse(text: &str) -> Result<ConfigLayer, AxError> {
        let file: ConfigFile = toml::from_str(text).map_err(|err| refuse(err.to_string()))?;
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
                Some(SandboxLimits {
                    shell: section.shell,
                    fuel: section
                        .fuel
                        .unwrap_or(kernel::consts_policy::SANDBOX_FUEL_DEFAULT),
                    mounts,
                    env_passthrough,
                })
            }
        };
        let mcp = match file.mcp {
            None => None,
            Some(entries) => {
                let mut servers: Vec<McpServer> = Vec::new();
                for entry in entries {
                    let label = ServerLabel::parse(&entry.label)
                        .map_err(|err| refuse(format!("{}: {}", entry.label, err.recovery())))?;
                    // One transport or the other, never both and never
                    // neither: a row that names a command and a url is a
                    // row whose reader has to guess which one was meant.
                    let transport = match (entry.command.as_deref(), entry.url.as_deref()) {
                        (Some(command), None) if !command.trim().is_empty() => {
                            McpTransport::Stdio {
                                command: command.to_owned(),
                                args: entry.args,
                            }
                        }
                        (None, Some(url)) if !url.trim().is_empty() => McpTransport::Http {
                            url: url.to_owned(),
                            header: entry.header,
                        },
                        (Some(_), Some(_)) => {
                            return Err(refuse(format!(
                                "{}: a server is reached by a command or by a url, not both",
                                label.as_str()
                            )));
                        }
                        _ => {
                            return Err(refuse(format!(
                                "{}: an mcp server needs a command to start or a url to reach",
                                label.as_str()
                            )));
                        }
                    };
                    // Two servers under one label would put one tool name
                    // in front of two processes, and that is a routing
                    // mistake rather than a preference.
                    if servers.iter().any(|held| held.label == label) {
                        return Err(refuse(format!(
                            "{}: two servers are named the same in one layer",
                            label.as_str()
                        )));
                    }
                    servers.push(McpServer { label, transport });
                }
                Some(servers)
            }
        };
        Ok(ConfigLayer {
            effort: file.model.effort,
            sandbox,
            mcp,
        })
    }

    #[must_use]
    pub fn effort(&self) -> Option<Effort> {
        self.effort
    }

    #[must_use]
    pub fn sandbox(&self) -> Option<&SandboxLimits> {
        self.sandbox.as_ref()
    }

    #[must_use]
    pub fn mcp(&self) -> Option<&[McpServer]> {
        self.mcp.as_deref()
    }
}

/// Resolves the three layers into the snapshot a run is frozen with.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse. A missing file is not an error: it is the
/// ordinary case.
pub fn load(city_root: &Path, addr: &Address) -> Result<FrozenConfig, AxError> {
    let city = read_layer(&path(city_root, addr, Layer::City)?)?;
    let building_path = path(city_root, addr, Layer::Building)?;
    let building = read_layer(&building_path)?;
    // An address that *is* its building has two rungs, not three: the
    // same file counted twice would suggest it can override itself.
    let resident_path = path(city_root, addr, Layer::Resident)?;
    let resident = if resident_path == building_path {
        ConfigLayer::default()
    } else {
        read_layer(&resident_path)?
    };

    Ok(kernel::freeze(
        &LayeredValue::default(),
        &LayeredValue::default(),
        &LayeredValue {
            city: city.effort(),
            building: building.effort(),
            resident: resident.effort(),
        },
        &LayeredValue {
            city: city.sandbox().cloned(),
            building: building.sandbox().cloned(),
            resident: resident.sandbox().cloned(),
        },
        &LayeredValue {
            city: city.mcp().map(<[McpServer]>::to_vec),
            building: building.mcp().map(<[McpServer]>::to_vec),
            resident: resident.mcp().map(<[McpServer]>::to_vec),
        },
    ))
}

fn read_layer(path: &Path) -> Result<ConfigLayer, AxError> {
    match std::fs::read_to_string(path) {
        // Three files can fail; the refusal says which one did.
        Ok(text) => ConfigLayer::parse(&text)
            .map_err(|err| refuse(format!("{}: {}", path.display(), err.subject()))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(ConfigLayer::default()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a configuration layer",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions; a configuration that exists is read")),
    }
}

/// One refusal shape for every way a layer can fail to be read, so the
/// recovery line is written once and cannot drift between callers.
fn refuse(subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read a configuration layer", subject).with_recovery(
        "this version reads three sections: `[model] effort = \"low|medium|high|xhigh|max\"`, \
         `[sandbox] shell = <bool>, fuel = <integer>, mounts = [<path>], \
         env_passthrough = [<variable name>]`, and \
         `[[mcp]] label = <lowercase>, and either command = <program> with args = [<argument>]          or url = <https url> with an optional header = \"Name: value\"`",
    )
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    model: ModelSection,
    #[serde(default)]
    sandbox: Option<SandboxSection>,
    #[serde(default)]
    mcp: Option<Vec<McpSection>>,
}

/// One `[[mcp]]` entry, read as written rather than as parsed types:
/// the label's grammar is `kernel`'s answer, and a deserializer that
/// enforced it here would be the second place that rule lives.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct McpSection {
    label: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    header: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SandboxSection {
    #[serde(default)]
    shell: bool,
    #[serde(default)]
    fuel: Option<u64>,
    #[serde(default)]
    mounts: Vec<String>,
    #[serde(default)]
    env_passthrough: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelSection {
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
