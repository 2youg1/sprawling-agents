// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one layer that is the person's rather than a city's
//! (sprawling-SPEC.md section 8-77).
//!
//! What somebody settled about how they read their cities — the
//! language, the appearance, the chords they rebound — lives in
//! `<home>/.sprawling/config.toml`, outside every city. A city carried
//! to another machine therefore arrives without it, and a second
//! browser on this machine is drawn from it rather than from whatever
//! the first browser happened to cache.
//!
//! **One record, one grammar.** The `[ui]` section is
//! `channels::PreferencesAnswer` serialised, so the keys the file may
//! hold and the fields the answer states are one declaration: this
//! module reads and writes, and states nothing about what a preference
//! is. What a named change does to the record is the patch's own rule
//! and lives with the patch.
//!
//! **Every other section is left as its author left it.** The write is
//! a read-modify-write of a file a person also edits by hand, held and
//! replaced whole through `city::document` — the one implementation of
//! whole-or-nothing writing in this tree — so a reader arriving during
//! a write finds the version before it or the version after it, and
//! two screens settling different facts cannot drop each other's.

use std::path::{Path, PathBuf};

use channels::{PreferencePatch, PreferencesAnswer};
use kernel::{AxCode, AxError};

use crate::home::Home;

/// The section of the person's file these live under. Written once:
/// the reader and the writer name it, and a section written under one
/// spelling and read under another is a setting that never takes
/// effect.
const UI: &str = "ui";

/// Everything this person settled, as their file states it.
///
/// A file nobody has written yet is not a failure: it is somebody who
/// has settled nothing, and the postures this build draws are the
/// answer.
///
/// # Errors
/// `E_PATH_NOT_FOUND` when no home directory can be found,
/// `E_CONFIG_INVALID` for a file that does not parse or holds a key
/// this version does not read, and `E_STORAGE_FATAL` for a file that
/// exists and cannot be read.
pub(crate) fn read() -> Result<PreferencesAnswer, AxError> {
    stated(&file()?)
}

/// Lands one named change in this person's file and leaves every other
/// section as they wrote it.
///
/// # Errors
/// Everything [`read`] returns, and `E_STORAGE_FATAL` for a file that
/// cannot be replaced.
pub(crate) fn put(patch: PreferencePatch) -> Result<(), AxError> {
    land(&file()?, patch)
}

/// The whole of the write, against a named file: read it, land the
/// change, put it back. Held while it reads and writes, so a second
/// screen settling another fact waits rather than deciding from an
/// original that is already stale.
fn land(file: &Path, patch: PreferencePatch) -> Result<(), AxError> {
    city::edit_document(file, |held| {
        let mut document = document(file)?;
        let mut settled = section(&document, file)?;
        settled.apply(patch);
        document.insert(UI.to_owned(), rendered(&settled, file)?);
        let text = toml::to_string_pretty(&document).map_err(|err| invalid(file, &err))?;
        held.replace(text.as_bytes())
    })
}

/// Where this person's file is. The home directory is `home::Home`'s
/// answer and the name under it is `Home::person_config`'s, so nothing
/// here joins a path.
fn file() -> Result<PathBuf, AxError> {
    Ok(Home::detect()?.person_config())
}

fn stated(file: &Path) -> Result<PreferencesAnswer, AxError> {
    section(&document(file)?, file)
}

/// The whole file as a table, and an empty table for a file nobody has
/// written. Read whole so that a write puts back the sections this
/// build does not read.
fn document(file: &Path) -> Result<toml::Table, AxError> {
    match std::fs::read_to_string(file) {
        Ok(text) => toml::from_str(&text).map_err(|err| invalid(file, &err)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(toml::Table::new()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read this person's preferences",
            format!("{}: {err}", file.display()),
        )
        .with_recovery(
            "fix that file's permissions; the preferences it holds are read at every start",
        )),
    }
}

/// The `[ui]` section as the record it is the serialisation of. An
/// absent section states nothing, which is the ordinary case until
/// somebody changes a setting.
fn section(document: &toml::Table, file: &Path) -> Result<PreferencesAnswer, AxError> {
    match document.get(UI) {
        None => Ok(PreferencesAnswer::default()),
        Some(section) => section
            .clone()
            .try_into()
            .map_err(|err| invalid(file, &format!("[{UI}]: {err}"))),
    }
}

fn rendered(settled: &PreferencesAnswer, file: &Path) -> Result<toml::Value, AxError> {
    toml::Value::try_from(settled).map_err(|err| invalid(file, &err))
}

/// One refusal for every way this file can fail to be understood, so
/// the recovery line is written once. The keys are not spelled again
/// here: `PreferencesAnswer` is what states them, and a second list
/// would be the one nobody updates.
fn invalid(file: &Path, why: &impl std::fmt::Display) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read this person's preferences",
        format!("{}: {why}", file.display()),
    )
    .with_recovery(
        "fix the `[ui]` section by hand, or delete it and choose again on the settings page",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use channels::{Chord, Lang};

    /// The property `adversary`'s fourth world drives over the wire,
    /// held here against the file itself: after any sequence of
    /// changes, what the file states and what the answer states are
    /// one record.
    #[test]
    fn what_the_file_states_and_what_the_answer_states_are_one_record() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let mut expected = PreferencesAnswer::default();
        for patch in [
            PreferencePatch::Welcomed(true),
            PreferencePatch::Lang(Lang::Zh),
            PreferencePatch::Chord(Chord {
                action: "go.city".to_owned(),
                spelled: "accel+2".to_owned(),
            }),
            PreferencePatch::Panel(false),
            PreferencePatch::Lang(Lang::En),
        ] {
            expected.apply(patch.clone());
            land(&file, patch).unwrap();
            assert_eq!(stated(&file).unwrap(), expected);
        }
        assert_eq!(expected.lang, Some(Lang::En));
    }

    /// A section a person wrote that this build does not read is still
    /// theirs: a preference written from a page must not delete it.
    #[test]
    fn a_section_this_build_does_not_read_survives_a_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "[theirs]\nkept = \"yes\"\n").unwrap();
        land(&file, PreferencePatch::Welcomed(true)).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("kept"), "{text}");
        assert!(stated(&file).unwrap().welcomed);
    }

    /// A file this build cannot read is not overwritten: the person
    /// who wrote it is the one who can fix it, and a page that
    /// silently replaced it would take their work away.
    #[test]
    fn a_file_that_does_not_parse_is_refused_rather_than_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "[ui]\nlang = \"fr\"\n").unwrap();
        let err = stated(&file).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(err.subject().contains("[ui]"), "{}", err.subject());
    }
}
