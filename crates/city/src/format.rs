// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `format` number a configuration file may carry, and the ladder
//! that brings an older file forward (`crates/city/spec/Format.lean`
//! §8-42).
//!
//! A file without `format` is format 1, so a file nobody migrated is
//! never written to: the previous release refuses `format` as an
//! unknown key in `CONFIG.toml` and `RULES.toml`, and a person who goes
//! back to it must still find every file it can read. A file stating a
//! format newer than this build's is refused whole, with a recovery that
//! names the upgrade, before any other key is judged — otherwise the
//! first new key would be refused as a misspelling.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, B3Hash};

/// The key every versioned file states its format under.
pub const FORMAT_KEY: &str = "format";

/// A file this product versions with a `format` number.
///
/// Exhaustive on purpose: a change that versions another kind of file
/// adds an arm, and every `match` below then asks what its current
/// format is and how its format is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatFile {
    /// The person's own `~/.sprawling/config.toml`.
    PersonConfig,
    /// A `CONFIG.toml` at the city's, a building's or a room's layer.
    LayerConfig,
    /// A building's `RULES.toml`.
    Rules,
}

/// One format of one kind of file. Never zero: format 1 is what a file
/// without the key is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Format(NonZeroU32);

impl Format {
    /// What a file that states no format is.
    pub const FIRST: Format = Format(NonZeroU32::MIN);

    /// The number as a file spells it.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// The format after this one, or `None` past the last number.
    fn next(self) -> Option<Format> {
        self.0.checked_add(1).map(Format)
    }
}

impl FormatFile {
    /// The format this build writes and reads.
    ///
    /// Raising one of these is a change that also registers a [`Rung`]
    /// from the previous number, or every older file of that kind would
    /// be refused at the next start.
    #[must_use]
    pub const fn current(self) -> Format {
        match self {
            FormatFile::PersonConfig | FormatFile::LayerConfig | FormatFile::Rules => Format::FIRST,
        }
    }

    /// The format `text` states, judging nothing else in it; `None`
    /// for text that is not TOML at all.
    ///
    /// Text that does not parse is left to the file's own reader, whose
    /// refusal names the file and who may change it; a refusal here
    /// would replace those words with a generic one.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for a `format` that is not a positive integer.
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

/// What every reader of a versioned file calls on its text before it
/// judges the file's keys, so a file from a newer build is refused for
/// being newer rather than for the first key this build does not know.
///
/// # Errors
/// `E_CONFIG_INVALID` when the file states a format newer than this
/// build's (the recovery names the upgrade), a format older than this
/// build's that nothing brought forward, a `format` that is not a
/// positive integer. Text that is not TOML is admitted here and refused
/// by the reader in its own words.
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

/// One step of the ladder: a file of `file` at format `from` becomes the
/// same file at the next format.
///
/// `rewrite` returns the whole new text, including its new `format`
/// line, because only the rung knows where in the file that line goes.
#[derive(Debug, Clone, Copy)]
pub struct Rung {
    pub file: FormatFile,
    pub from: Format,
    pub rewrite: fn(&str) -> Result<String, AxError>,
}

/// Every rung, in no particular order; [`bring_forward`] picks each by
/// its file and its `from`. Empty while every kind of file is still at
/// its first format.
const RUNGS: &[Rung] = &[];

/// What [`bring_forward`] found and did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forwarded {
    /// There is no file at that path.
    Absent,
    /// The file was already at this build's format and was not written.
    Current,
    /// The file was rewritten from `from` to `to`; its original bytes
    /// are at `backup`.
    Moved {
        from: Format,
        to: Format,
        backup: PathBuf,
    },
}

/// Brings the file at `path` to this build's format, keeping its
/// original bytes under `backups` first.
///
/// A file already at the current format is not written to, which is
/// what lets the previous release keep reading every file nothing
/// needed to change.
///
/// # Errors
/// `E_CONFIG_INVALID` for a file newer than this build or a format no
/// rung leads from; `E_STORAGE_FATAL` for a file or backup that cannot
/// be read or written, and for a backup of a different text already
/// standing where this one would go; whatever a rung refuses.
pub fn bring_forward(path: &Path, file: FormatFile, backups: &Path) -> Result<Forwarded, AxError> {
    let ladder = Ladder {
        to: file.current(),
        rungs: RUNGS,
    };
    climb(path, file, &ladder, backups)
}

/// One file opening the city brought forward or could not: its
/// `outcome` is `Forwarded::Moved` or the refusal, never `Absent` or
/// `Current`, because a file nothing happened to needs no line.
#[derive(Debug)]
pub struct Forwarding {
    pub path: PathBuf,
    pub outcome: Result<Forwarded, AxError>,
}

/// Brings every versioned file the city holds to this build's format:
/// the city's, each building's and each room's `CONFIG.toml`, and each
/// building's `RULES.toml`, as [`crate::check`] lists them.
///
/// A file that is refused does not stop the others, because its own
/// reader refuses it again when it is read and the rest should not stay
/// behind with it. While no rung is registered this reads and writes
/// nothing beyond the reads.
///
/// # Errors
/// Propagates a failure to list the buildings or rooms; each file's own
/// refusal is in its [`Forwarding`].
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

/// [`bring_city_forward`] against the ladder `ladder_for` names for each
/// kind of file.
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

/// The format a climb ends at and the rungs it may take: this build's
/// in production, a test's own where a test needs a format past the
/// current one.
struct Ladder<'a> {
    to: Format,
    rungs: &'a [Rung],
}

/// [`bring_forward`] against a named ladder, so the climb is one
/// implementation whether the ladder is this build's or a test's.
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
    // A file that does not parse is not brought anywhere: its reader
    // refuses it, in words that name the file, when it is next read.
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

/// Writes `bytes` to the backup of `path` at `from`, and answers where.
///
/// The name carries a digest of the directory the file lives in, because
/// every building has a `RULES.toml` and every layer a `CONFIG.toml`,
/// and one directory of backups holds all of them. The backup lands by
/// one rename, so an interrupted write leaves no partial text under the
/// backup's name for a retry to mistake for a different backup.
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
