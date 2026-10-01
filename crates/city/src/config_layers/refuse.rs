// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One refusal shape for every way a layer can fail to be read, so the
//! recovery line is written once and cannot drift between callers.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4.

use std::path::Path;

use kernel::layout::CONFIG_FILE;
use kernel::{AxCode, AxError, RESERVED_PREFIX};

use super::remote::REMOTE_KEY;
use super::resident::{HARNESS_KEY, MODEL_NAME_KEY};
use super::shelves::SHELVES_KEY;

/// What a person does when a key or a value is not one this build reads.
/// The message names the key, so the sentence does not.
const CHANGE_THE_VALUE: &str = "change the value the message names, or take that key out";

/// The refusal a layer answers with when a value it read is not one this
/// build accepts. `subject` is the caller's: which file, and which value.
pub(super) fn refuse(subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read a configuration layer", subject)
        .with_recovery(CHANGE_THE_VALUE)
}

/// The refusal for text serde itself will not read.
///
/// serde's message already names the key it did not recognise and the
/// keys that table accepts, so the list of what this build reads has one
/// home there. This function adds only where to look: the table the
/// parser stopped in, spelled the way the document spells it, taken from
/// the span serde reports rather than typed out here.
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
    AxError::failure(AxCode::ConfigInvalid, "read a configuration layer", subject)
        .with_recovery(recovery)
}

/// A table only the city's own layer may state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CityOnly {
    /// A shelf is mounted for every building at once, so a building or
    /// a room that names one would admit a directory nobody who keeps
    /// this city chose (`crates/city/spec/Library.lean` §8-8).
    Shelves,
    /// The remote door opens onto the whole city, so a building or a
    /// room that chose its route would decide for every other one how
    /// the outside comes in (`crates/city/spec/ConfigLayers/Remote.lean` §8-39).
    Remote,
}

impl CityOnly {
    fn key(self) -> &'static str {
        match self {
            CityOnly::Shelves => SHELVES_KEY,
            CityOnly::Remote => REMOTE_KEY,
        }
    }

    fn because(self) -> &'static str {
        match self {
            CityOnly::Shelves => "a shelf is mounted for every building at once",
            CityOnly::Remote => "the remote door opens onto the whole city",
        }
    }
}

/// The refusal for a city-only table written below the city layer,
/// refused where it was written rather than parsed and dropped.
pub(super) fn below_city(file: &Path, table: CityOnly) -> AxError {
    let key = table.key();
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a configuration layer",
        format!("{}: `{key}`", file.display()),
    )
    .with_recovery(format!(
        "move `{key}` into the city root's `{RESERVED_PREFIX}/{CONFIG_FILE}`: {}",
        table.because()
    ))
}

/// The refusal for one layer that names a model and a harness.
///
/// The model is the record a session wrote at this address, and `/new`
/// forgets it; the harness is a person's setting. Which of the two the
/// layer meant is not a reader's to guess.
pub(super) fn two_residents(model: &str, harness: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a configuration layer",
        format!("{MODEL_NAME_KEY} = \"{model}\" beside {HARNESS_KEY} = \"{harness}\""),
    )
    .with_recovery(format!(
        "keep one: `/new` forgets the model a session wrote here, or take {HARNESS_KEY} out"
    ))
}

/// The table header the parser was inside, as the document spells it.
///
/// The span points at the key, so only whole lines above it are
/// searched: a top-level key has no table above it, and the line the
/// key sits on may itself begin with a bracket.
fn enclosing_table(span: &std::ops::Range<usize>, text: &str) -> Option<String> {
    let before_span = text.get(..span.start)?;
    let newline = before_span.rfind('\n')?;
    text.get(..newline)?
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('['))
        .map(|line| line.trim().to_owned())
}
