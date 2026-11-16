// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, B3Hash};

const ID_MAX: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgentId(String);

impl AgentId {

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

    pub(super) fn official(word: &'static str) -> AgentId {
        AgentId(word.to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSource {

    Registry,

    Detected,

    Pasted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEntry {
    pub id: AgentId,
    pub name: String,
    pub source: AgentSource,
    pub launch: Launch,
    pub version: Option<String>,
    pub licence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,

    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pin {

    Exact,

    Floating,

    Unknown,
}

impl Launch {

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Launcher {
    Npx,
    Uvx,
}

impl Launcher {

    pub(super) const fn program(self) -> &'static str {
        match self {
            Launcher::Npx if cfg!(windows) => "npx.cmd",
            Launcher::Npx => "npx",
            Launcher::Uvx => "uvx",
        }
    }
}

pub(super) fn program_stem(program: &str) -> &str {
    let name = program.rsplit(['/', '\\']).next().unwrap_or(program);
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        Some(_) | None => name,
    }
}

fn launcher(program: &str) -> Option<Launcher> {
    match program_stem(program).to_ascii_lowercase().as_str() {
        "npx" => Some(Launcher::Npx),
        "uvx" => Some(Launcher::Uvx),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consented {
    entry: AgentEntry,
}

impl Consented {

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
