// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One ACP agent entry, and the consent that lets it start
//! (`crates/agent_protocols/Spec.lean` §8-19, D16).
//!
//! An entry is only an offer: the catalog, the paste parser and the
//! detection produce entries, and nothing here starts a process. The one
//! door that does, `HarnessProcess::start`, takes a [`Consented`], whose
//! field is private and whose two constructors are the two ways a person
//! consents (`crates/agent_protocols/spec/Harness/Consent.lean`).

use kernel::{AxCode, AxError, B3Hash};

/// The longest agent id, in bytes.
const ID_MAX: usize = 64;

/// The id an agent entry travels under: what `[resident] harness` names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgentId(String);

impl AgentId {
    /// Reads an id: lowercase ASCII letters, digits, `-`, `_` and `.`,
    /// starting with a letter or a digit, at most 64 bytes. Uppercase is
    /// folded, because a pasted program name is written either way.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for an empty id, one too long, and a character
    /// outside the set.
    pub fn parse(text: &str) -> Result<AgentId, AxError> {
        let id = text.trim().to_ascii_lowercase();
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_.".contains(c);
        let starts = id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        if starts && id.len() <= ID_MAX && id.chars().all(allowed) {
            return Ok(AgentId(id));
        }
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read an agent id",
            text.to_owned(),
        )
        .with_recovery(
            "name the agent with letters, digits, `-`, `_` or `.`, starting with a letter or a \
             digit, at most 64 characters",
        ))
    }

    /// A built-in entry's word, which `OFFICIAL` spells in the id grammar.
    pub(super) fn official(word: &'static str) -> AgentId {
        AgentId(word.to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where an agent entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSource {
    /// The ACP registry's catalog, as this build shipped it.
    Registry,
    /// Found on this machine.
    Detected,
    /// Pasted by the person.
    Pasted,
}

/// One agent the person may consent to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEntry {
    pub id: AgentId,
    pub name: String,
    pub source: AgentSource,
    pub launch: Launch,
    pub version: Option<String>,
    pub licence: Option<String>,
}

/// The command that starts an agent as an ACP agent on stdio: what the
/// person consents to, byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    /// Name before value, in the order the entry lists them. A value may
    /// be a `secret:realm/name` reference, redeemed when the child starts.
    pub env: Vec<(String, String)>,
}

/// How exactly a launch names the agent's version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pin {
    /// One exact version: `pkg@1.2.3`, `pkg==1.2.3`.
    Exact,
    /// Whatever the launcher resolves: no version, or `latest`.
    Floating,
    /// A program on this machine, whose version nothing here read.
    Unknown,
}

impl Launch {
    /// The digest a consent binds (D16): the program, every argument and
    /// every environment pair, each written as a netstring (its length in
    /// decimal, a colon, its bytes), so `["a b"]` and `["a", "b"]` never
    /// share a digest.
    #[must_use]
    pub fn digest(&self) -> B3Hash {
        let mut bytes = Vec::new();
        let fields = std::iter::once(self.program.as_str())
            .chain(std::iter::once("\0args"))
            .chain(self.args.iter().map(String::as_str))
            .chain(std::iter::once("\0env"))
            .chain(
                self.env
                    .iter()
                    .flat_map(|(name, value)| [name.as_str(), value.as_str()]),
            );
        for field in fields {
            bytes.extend_from_slice(format!("{}:", field.len()).as_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
        B3Hash::digest(&bytes)
    }

    /// The command line the consent card shows: each word as it is, and
    /// a word with a space or a quote in it inside double quotes.
    #[must_use]
    pub fn preview(&self) -> String {
        std::iter::once(&self.program)
            .chain(&self.args)
            .map(|word| {
                if word.is_empty() || word.contains([' ', '\t', '"']) {
                    format!("\"{}\"", word.replace('"', "\\\""))
                } else {
                    word.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Whether the package this launch fetches is pinned to one version.
    #[must_use]
    pub fn pin(&self) -> Pin {
        let fetched = match launcher(&self.program) {
            Some(Launcher::Npx | Launcher::Uvx) => {
                self.args.iter().find(|arg| !arg.starts_with('-'))
            }
            None => return Pin::Unknown,
        };
        let version = fetched.and_then(|package| {
            package
                .split_once("==")
                .map(|(_, version)| version)
                .or_else(|| {
                    // A scoped npm package starts with `@`; its version
                    // follows the last `@`.
                    package
                        .rsplit_once('@')
                        .filter(|(name, _)| !name.is_empty())
                        .map(|(_, version)| version)
                })
        });
        match version {
            Some(version) if version.starts_with(|c: char| c.is_ascii_digit()) => Pin::Exact,
            Some(_) | None => Pin::Floating,
        }
    }
}

/// The two package runners a registry entry names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Launcher {
    Npx,
    Uvx,
}

impl Launcher {
    /// The name the search path is asked for. Node installs `npx` as a
    /// batch file on Windows, which a process cannot be started from under
    /// its bare name.
    pub(super) const fn program(self) -> &'static str {
        match self {
            Launcher::Npx if cfg!(windows) => "npx.cmd",
            Launcher::Npx => "npx",
            Launcher::Uvx => "uvx",
        }
    }
}

/// The package runner a program is, read from its file name.
fn launcher(program: &str) -> Option<Launcher> {
    let name = std::path::Path::new(program)
        .file_stem()
        .and_then(|stem| stem.to_str())?
        .to_ascii_lowercase();
    match name.as_str() {
        "npx" => Some(Launcher::Npx),
        "uvx" => Some(Launcher::Uvx),
        _ => None,
    }
}

/// An entry the person consented to: the only thing a child is started
/// from (`crates/agent_protocols/spec/Harness/Consent.lean`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consented {
    entry: AgentEntry,
}

impl Consented {
    /// The consent the card recorded: `digest` is what the person pressed,
    /// and the entry is held to it.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when the digest recomputed from the entry is not
    /// the recorded one: the launch changed after the person consented.
    pub fn given(entry: AgentEntry, digest: &B3Hash) -> Result<Consented, AxError> {
        if entry.launch.digest() == *digest {
            return Ok(Consented { entry });
        }
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "start an ACP agent",
            format!(
                "{}: the command is not the one consented to",
                entry.id.as_str()
            ),
        )
        .with_recovery(
            "add the agent again from the ACP page, which shows the command it will run; the row \
             in CONFIG.toml changed after it was added",
        ))
    }

    /// A row the person wrote into the city's `CONFIG.toml` with their own
    /// hands, and a built-in entry a person named there: that file is the
    /// consent, as it is for `[[mcp]]` and `[remote]`.
    #[must_use]
    pub fn written_by_hand(entry: AgentEntry) -> Consented {
        Consented { entry }
    }

    #[must_use]
    pub fn entry(&self) -> &AgentEntry {
        &self.entry
    }
}

#[cfg(test)]
mod tests;
