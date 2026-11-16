// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::CONFIG_FILE;
use kernel::{AxCode, AxError, RESERVED_PREFIX};

use super::agents::AGENT_KEY;
use super::catalog::CATALOG_KEY;
use super::ladder::Layer;
use super::mcp::MCP_REFERENCE_FIELD;
use super::remote::REMOTE_KEY;
use super::resident::{HARNESS_KEY, MODEL_NAME_KEY};
use super::search::SEARCH_KEY;
use super::shelves::{SHELVES_KEY, SKILL_PINS_KEY};

const CHANGE_THE_VALUE: &str = "change the value the message names, or take that key out";

pub(super) const READ_A_LAYER: &str = "read a configuration layer";

pub(super) fn refuse(subject: String) -> AxError {
    refuse_with(subject, CHANGE_THE_VALUE)
}

pub(super) fn refuse_with(subject: String, recovery: impl Into<String>) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, READ_A_LAYER, subject).with_recovery(recovery)
}

pub(super) fn unreadable(text: &str, err: &toml::de::Error) -> AxError {
    let table = err.span().and_then(|span| enclosing_table(&span, text));
    let subject = match &table {
        Some(table) => format!("{table}: {}", err.message()),
        None => err.message().to_owned(),
    };
    let recovery = match &table {
        Some(table) => format!("under `{table}`, {CHANGE_THE_VALUE}"),
        None => CHANGE_THE_VALUE.to_owned(),
    };
    refuse_with(subject, recovery)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Confined {

    Shelves,

    Remote,

    Search,

    Agents,

    Catalog,

    McpReferences,

    SkillPins,
}

impl Confined {

    pub(super) fn nearest(self) -> Layer {
        match self {
            Confined::Shelves | Confined::Remote | Confined::Agents => Layer::City,
            Confined::Search
            | Confined::Catalog
            | Confined::McpReferences
            | Confined::SkillPins => Layer::Building,
        }
    }

    pub(super) fn reached_from(self, rung: Layer) -> bool {
        rung <= self.nearest()
    }

    fn key(self) -> String {
        match self {
            Confined::Shelves => SHELVES_KEY.to_owned(),
            Confined::Remote => REMOTE_KEY.to_owned(),
            Confined::Search => SEARCH_KEY.to_owned(),
            Confined::Agents => AGENT_KEY.to_owned(),
            Confined::Catalog => CATALOG_KEY.to_owned(),
            Confined::McpReferences => format!("[[{MCP_REFERENCE_FIELD}]]"),
            Confined::SkillPins => SKILL_PINS_KEY.to_owned(),
        }
    }

    fn because(self) -> &'static str {
        match self {
            Confined::Shelves => "a shelf is mounted for every building at once",
            Confined::Remote => "the remote door opens onto the whole city",
            Confined::Agents => "an agent runs as the person, for every room that names it",
            Confined::Catalog => {
                "the tool array is cached ahead of the system prompt, and rooms of one \
                 building that differed would split its cache"
            }
            Confined::Search => {
                "the settings page shows the city's and each building's choice, and a room's \
                would govern runs it never shows"
            }
            Confined::McpReferences => {
                "a building references a server definition, and a room keeps its selection in \
                its own CATALOG.toml"
            }
            Confined::SkillPins => {
                "a building pins its Reading Room, and a room keeps those pins in CATALOG.toml"
            }
        }
    }
}

pub(super) fn too_near(file: &Path, table: Confined) -> AxError {
    let key = table.key();
    let into = match table.nearest() {
        Layer::City => format!("the city root's `{RESERVED_PREFIX}/{CONFIG_FILE}`"),
        Layer::Building | Layer::Resident => {
            format!("the building's or the city root's `{RESERVED_PREFIX}/{CONFIG_FILE}`")
        }
    };
    refuse_with(
        format!("{}: `{key}`", file.display()),
        format!("move `{key}` into {into}: {}", table.because()),
    )
}

pub(super) fn two_residents(model: &str, harness: &str) -> AxError {
    refuse_with(
        format!("{MODEL_NAME_KEY} = \"{model}\" beside {HARNESS_KEY} = \"{harness}\""),
        format!(
            "keep one: `/new` forgets the model a session wrote here, or take {HARNESS_KEY} out"
        ),
    )
}

fn enclosing_table(span: &std::ops::Range<usize>, text: &str) -> Option<String> {
    let before_span = text.get(..span.start)?;
    let newline = before_span.rfind('\n')?;
    text.get(..newline)?
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('['))
        .map(|line| line.trim().to_owned())
}
