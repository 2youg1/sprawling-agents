// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, B3Hash};

pub const FORMAT_KEY: &str = "format";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatFile {

    PersonConfig,

    LayerConfig,

    Rules,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Format(NonZeroU32);

impl Format {

    pub const FIRST: Format = Format(NonZeroU32::MIN);

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    fn next(self) -> Option<Format> {
        self.0.checked_add(1).map(Format)
    }
}

impl FormatFile {

    #[must_use]
    pub const fn current(self) -> Format {
        match self {
            FormatFile::PersonConfig | FormatFile::LayerConfig | FormatFile::Rules => Format::FIRST,
        }
    }

    pub fn stated(self, text: &str) -> Result<Option<Format>, AxError> {
        match self {
            FormatFile::PersonConfig | FormatFile::LayerConfig | FormatFile::Rules => {
                let Ok(table) = toml::from_str::<toml::Table>(text) else {
                    return Ok(None);
                };
                match table.get(FORMAT_KEY) {
                    None => Ok(Some(Format::FIRST)),
                    Some(toml::Value::Integer(n)) => number(*n).map(Some),
                    Some(other) => Err(not_a_number(other)),
                }
            }
        }
    }
}

pub fn admit_format(file: FormatFile, text: &str) -> Result<(), AxError> {
    let Some(stated) = file.stated(text)? else {
        return Ok(());
    };
    match stated.cmp(&file.current()) {
        std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(newer(stated, file.current())),
        std::cmp::Ordering::Less => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read a configuration file",
            format!(
                "format {} is older than the format {} this build reads",
                stated.get(),
                file.current().get()
            ),
        )
        .with_recovery("restart the city: it brings older files forward when it opens")),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rung {
    pub file: FormatFile,
    pub from: Format,
    pub rewrite: fn(&str) -> Result<String, AxError>,
}

const RUNGS: &[Rung] = &[];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forwarded {

    Absent,

    Current,

    Moved {
        from: Format,
        to: Format,
        backup: PathBuf,
    },
}

pub fn bring_forward(path: &Path, file: FormatFile, backups: &Path) -> Result<Forwarded, AxError> {
    let ladder = Ladder {
        to: file.current(),
        rungs: RUNGS,
    };
    climb(path, file, &ladder, backups)
}

#[derive(Debug)]
pub struct Forwarding {
    pub path: PathBuf,
    pub outcome: Result<Forwarded, AxError>,
}

pub fn bring_city_forward(city_root: &Path, backups: &Path) -> Result<Vec<Forwarding>, AxError> {
    climb_city(
        city_root,
        |file| Ladder {
            to: file.current(),
            rungs: RUNGS,
        },
        backups,
    )
}

fn climb_city<'a>(
    city_root: &Path,
    ladder_for: impl Fn(FormatFile) -> Ladder<'a>,
    backups: &Path,
) -> Result<Vec<Forwarding>, AxError> {
    Ok(crate::check::files(city_root)?
        .into_iter()
        .filter_map(|(path, kind)| kind.versioned().map(|file| (path, file)))
        .map(|(path, file)| {
            let outcome = climb(&path, file, &ladder_for(file), backups);
            Forwarding { path, outcome }
        })
        .filter(|forwarding| {
            !matches!(
                forwarding.outcome,
                Ok(Forwarded::Absent | Forwarded::Current)
            )
        })
        .collect())
}

struct Ladder<'a> {
    to: Format,
    rungs: &'a [Rung],
}

fn climb(
    path: &Path,
    file: FormatFile,
    ladder: &Ladder<'_>,
    backups: &Path,
) -> Result<Forwarded, AxError> {
    let to = ladder.to;
    let original = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Forwarded::Absent),
        Err(err) => return Err(storage("read a configuration file", path, &err)),
    };

    let Some(from) = file.stated(&original)? else {
        return Ok(Forwarded::Current);
    };
    match from.cmp(&to) {
        std::cmp::Ordering::Equal => return Ok(Forwarded::Current),
        std::cmp::Ordering::Greater => return Err(newer(from, to)),
        std::cmp::Ordering::Less => {}
    }
    let mut text = original.clone();
    let mut at = from;
    while at < to {
        let rung = ladder
            .rungs
            .iter()
            .find(|rung| rung.file == file && rung.from == at)
            .ok_or_else(|| missing_rung(file, at))?;
        text = (rung.rewrite)(&text)?;
        at = at.next().ok_or_else(|| missing_rung(file, at))?;
    }
    let backup = keep(path, from, original.as_bytes(), backups)?;
    crate::document::replace(path, text.as_bytes())?;
    Ok(Forwarded::Moved { from, to, backup })
}

fn keep(path: &Path, from: Format, bytes: &[u8], backups: &Path) -> Result<PathBuf, AxError> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let digest = B3Hash::digest(parent.to_string_lossy().as_bytes());
    let short: String = digest
        .as_bytes()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let backup = backups.join(format!("{short}-{name}.bak-{}", from.get()));
    crate::document::edit(&backup, |held| match std::fs::read(&backup) {
        Ok(standing) if standing == bytes => Ok(()),
        Ok(_) => Err(AxError::failure(
            AxCode::StorageFatal,
            "keep a configuration file's backup",
            format!("{}: a different backup is already there", backup.display()),
        )
        .with_recovery("move that backup somewhere else, then start the city again")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => held.replace(bytes),
        Err(err) => Err(storage("read an earlier backup", &backup, &err)),
    })?;
    Ok(backup)
}

fn number(raw: i64) -> Result<Format, AxError> {
    match u32::try_from(raw).map(NonZeroU32::new) {
        Ok(Some(n)) => Ok(Format(n)),
        Ok(None) | Err(_) => Err(not_a_number(&toml::Value::Integer(raw))),
    }
}

fn not_a_number(value: &toml::Value) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a configuration file's format",
        format!("{FORMAT_KEY} = {value} is not a positive integer"),
    )
    .with_recovery(format!(
        "delete the {FORMAT_KEY} line: a file without it is format 1"
    ))
}

fn newer(stated: Format, current: Format) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a configuration file",
        format!(
            "written by a newer sprawling (format {}; this build reads up to {})",
            stated.get(),
            current.get()
        ),
    )
    .with_recovery("upgrade sprawling to the version that wrote this file")
}

fn missing_rung(file: FormatFile, at: Format) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "bring a configuration file forward",
        format!("no step leads from format {} of {file:?}", at.get()),
    )
    .with_recovery("open this file with the sprawling that wrote it")
}

fn storage(action: &str, path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action.to_owned(),
        format!("{}: {err}", path.display()),
    )
    .with_recovery("fix that path's permissions or free disk space, then start the city again")
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
