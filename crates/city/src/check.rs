// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every TOML file a city holds, read by the parser that reads it at run
//! time, with the first refusal of each placed at a line and a column
//! (`crates/city/spec/Check.lean` §8-29).
//!
//! The parser's verdict is the verdict. Only when it refuses is the file
//! read a second time with the same shape, to take the span `toml`
//! reports; a refusal that comes after the shape was read has no span,
//! and gets no invented position.

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError};
use serde::de::DeserializeOwned;

use crate::config_layers::{ConfigFile, ConfigLayer, Layer};
use crate::policy::{RulesShape, evaluate, rules_path};
use crate::schedule::{Schedule, ScheduleFile, schedule_path};
use crate::watch::{Watch, WatchFile, watch_path};

/// Where in a file a refusal points, both counted from one. The column
/// counts characters, not bytes, which is what an editor jumps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// One file the city holds that its parser refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub at: Option<Position>,
    pub error: AxError,
}

/// What reading every file found: how many files were read, and the
/// refusals among them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub read: usize,
    pub findings: Vec<Finding>,
}

/// The kind of file, which names the parser that reads it.
enum Kind {
    Config,
    Rules(Address),
    Schedule,
    Watch,
}

/// Reads every TOML file the city holds.
///
/// # Errors
/// Propagates a failure to list the buildings or rooms, or to read a
/// file that exists: then whether the city has an error cannot be told.
pub fn check(city_root: &Path) -> Result<Report, AxError> {
    let mut files = vec![(CityLayout::new(city_root).city_config(), Kind::Config)];
    for building in crate::building::all(city_root)? {
        files.push((Layer::Building.file(city_root, &building)?, Kind::Config));
        files.push((
            rules_path(city_root, &building),
            Kind::Rules(building.clone()),
        ));
        for room in crate::room::all(city_root, &building)? {
            files.push((Layer::Resident.file(city_root, &room)?, Kind::Config));
        }
    }
    files.push((schedule_path(city_root), Kind::Schedule));
    files.push((watch_path(city_root), Kind::Watch));
    let mut report = Report::default();
    for (path, kind) in files {
        let Some(text) = read(&path)? else { continue };
        report.read = report.read.saturating_add(1);
        if let Err(error) = kind.parse(&text) {
            let at = kind.locate(&text);
            report.findings.push(Finding { path, at, error });
        }
    }
    Ok(report)
}

impl Kind {
    fn parse(&self, text: &str) -> Result<(), AxError> {
        match self {
            Kind::Config => ConfigLayer::parse(text).map(drop),
            Kind::Rules(addr) => evaluate(addr, text).map(drop),
            Kind::Schedule => Schedule::parse(text).map(drop),
            Kind::Watch => Watch::parse(text).map(drop),
        }
    }

    /// Where the shape this kind is read with stops reading `text`.
    fn locate(&self, text: &str) -> Option<Position> {
        match self {
            Kind::Config => span_of::<ConfigFile>(text),
            Kind::Rules(_) => span_of::<RulesShape>(text),
            Kind::Schedule => span_of::<ScheduleFile>(text),
            Kind::Watch => span_of::<WatchFile>(text),
        }
    }
}

fn span_of<Shape: DeserializeOwned>(text: &str) -> Option<Position> {
    let start = toml::from_str::<Shape>(text).err()?.span()?.start;
    let before = text.get(..start)?;
    let line_start = before
        .rfind('\n')
        .map_or(0, |newline| newline.saturating_add(1));
    Some(Position {
        line: before.matches('\n').count().saturating_add(1),
        column: before.get(line_start..)?.chars().count().saturating_add(1),
    })
}

/// A file's text, or nothing when the city never wrote it: an absent
/// file is the default, not an error.
fn read(path: &Path) -> Result<Option<String>, AxError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "check a city's files",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions, then check again")),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
#[path = "check/tests.rs"]
mod tests;
