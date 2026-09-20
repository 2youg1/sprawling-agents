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

use std::path::{Path, PathBuf};

use kernel::{
    Address, AxCode, AxError, Effort, FrozenConfig, LayeredValue, McpServer, McpTransport,
    SandboxLimits, ServerLabel,
};
use serde::Deserialize;

mod ladder;
mod write;

pub use ladder::Layer;
pub use write::{write_effort, write_mcp, write_sandbox};

use ladder::Ladder;

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
                            // A command answers on its own pipes, so a
                            // row that also names a stream is a row
                            // stating two transports.
                            if entry.transport.is_some() {
                                return Err(refuse(format!(
                                    "{}: a command is spoken to over its own pipes; `transport` \
                                     chooses between `http` and `sse` for a url",
                                    label.as_str()
                                )));
                            }
                            McpTransport::Stdio {
                                command: command.to_owned(),
                                args: entry.args,
                                env: listed(entry.env),
                            }
                        }
                        (None, Some(url)) if !url.trim().is_empty() => {
                            let url = url.to_owned();
                            let headers = listed(entry.headers);
                            // Absent means `http`: that is what a url
                            // reached by posting one message is, and it
                            // is what every row written before this key
                            // existed meant.
                            match entry.transport.unwrap_or(ReachedBy::Http) {
                                ReachedBy::Http => McpTransport::Http { url, headers },
                                ReachedBy::Sse => McpTransport::Sse { url, headers },
                            }
                        }
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
    Ok(kernel::freeze(
        // No layer of this file has a key for either clock concern
        // yet: writing one is refused where it is written, so nothing
        // on the ladder can state them.
        &LayeredValue::default(),
        &LayeredValue::default(),
        &ladder.resolve(ConfigLayer::effort),
        &ladder.resolve(|layer| layer.sandbox().cloned()),
        &ladder.resolve(|layer| layer.mcp().map(<[McpServer]>::to_vec)),
    ))
}

/// One refusal shape for every way a layer can fail to be read, so the
/// recovery line is written once and cannot drift between callers.
fn refuse(subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read a configuration layer", subject).with_recovery(
        "this version reads three sections: `[model] effort = \"low|medium|high|xhigh|max\"`, \
         `[sandbox] shell = <bool>, fuel = <integer>, mounts = [<path>], \
         env_passthrough = [<variable name>]`, and \
         `[[mcp]] label = <lowercase>, and either command = <program> with args = [<argument>] \
         and env = { NAME = \"value\" }, or url = <https url> with transport = \"http\"|\"sse\" \
         and headers = { Name = \"value\" }`; a value on either table may be a \
         `secret:realm/name` reference",
    )
}

/// A configured table as the wire carries it: name before value, in the
/// order a `BTreeMap` reads them, so two runs of the same file hand the
/// same list to the same server.
fn listed(table: std::collections::BTreeMap<String, String>) -> Vec<(String, String)> {
    table.into_iter().collect()
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
    /// What the child process is started with. A table rather than a
    /// list of `NAME=value` lines: the file states one name once, and
    /// nothing here has to split a string a person wrote.
    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: std::collections::BTreeMap<String, String>,
    /// Which of the two ways a url is reached. Absent means `http`.
    #[serde(default)]
    transport: Option<ReachedBy>,
}

/// The two ways a url answers. A closed set rather than free text, so a
/// misspelling is refused where it is written instead of becoming a
/// server nobody can reach.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReachedBy {
    Http,
    Sse,
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
