// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city calls the person and the Mayor: the identity area at
//! the top of `PREFERENCES.md` and `MAYOR.md` (city-SPEC.md 8-33).
//!
//! **One reader.** The page, a save, a dispatch and the prefix all take
//! the area through [`split`], so they cannot disagree about what a name
//! is or where the area stopped reading. An area that does not read is
//! refused with its line rather than read as "no name": a page that drew
//! the default name would let the next save write over the line the
//! person got wrong, and the body under it.
//!
//! **A name says what someone is called, never what they may do.** The
//! Mayor is `hall/mayor` under any name; nothing here reaches an address,
//! an actor or a permission.

use std::path::Path;

use kernel::consts_policy::HALL_MAYOR;
use kernel::{Address, AxCode, AxError, B3Hash};
use serde::{Deserialize, Serialize};

use crate::governed::Governed;

/// The line that opens and closes an identity area.
const FENCE: &str = "+++";
/// The longest name a person may give, in characters: long enough for a
/// full name in any script, short enough to sit in a message header.
pub const NAME_MAX_CHARS: usize = 64;

const USER_ID: &str = "user_id";
const IMPORTED_FROM: &str = "imported_from";
const NAME: &str = "name";

/// A name a person gave: one line, trimmed, 1 to [`NAME_MAX_CHARS`]
/// characters, no control characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

impl DisplayName {
    /// # Errors
    /// `E_CONFIG_INVALID` for a name that is empty once trimmed, longer
    /// than [`NAME_MAX_CHARS`], or holds a control character.
    pub fn parse(raw: &str) -> Result<DisplayName, AxError> {
        let trimmed = raw.trim();
        let count = trimmed.chars().count();
        if count == 0 || count > NAME_MAX_CHARS || trimmed.chars().any(char::is_control) {
            return Err(
                AxError::failure(AxCode::ConfigInvalid, "read a name", format!("{raw:?}"))
                    .with_recovery(format!(
                        "give a name of 1 to {NAME_MAX_CHARS} characters on one line"
                    )),
            );
        }
        Ok(DisplayName(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for DisplayName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DisplayName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        DisplayName::parse(&raw).map_err(|err| serde::de::Error::custom(err.subject()))
    }
}

/// What the two documents' identity areas state, read together because a
/// session freezes them together.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Naming {
    person: Option<DisplayName>,
    imported_from: Option<DisplayName>,
    about: String,
    mayor: Option<DisplayName>,
}

/// Where an identity area stopped reading: which document, which line of
/// it counted from the first, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub document: Governed,
    pub line: u32,
    pub why: String,
}

impl Unreadable {
    /// The refusal every caller that cannot go on without the names gives.
    #[must_use]
    pub fn into_ax(self) -> AxError {
        AxError::failure(
            AxCode::ConfigInvalid,
            "read the identity area",
            format!("{}:{}: {}", self.document.file(), self.line, self.why),
        )
        .with_recovery(
            "fix that line between the two +++ lines, or delete the identity area to fall back \
             on the default names",
        )
    }
}

/// One identity card's values. `None` removes the key, which puts the
/// default name back; `about: None` leaves the body as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingEdit {
    Person {
        user_id: Option<String>,
        imported_from: Option<String>,
        about: Option<String>,
    },
    Mayor {
        name: Option<String>,
    },
}

impl NamingEdit {
    /// The document this card writes.
    #[must_use]
    pub fn document(&self) -> Governed {
        match self {
            NamingEdit::Person { .. } => Governed::Preferences,
            NamingEdit::Mayor { .. } => Governed::Mayor,
        }
    }
}

/// What one save left: the names the city now has, and how long the
/// document it wrote is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingWritten {
    pub naming: Naming,
    pub bytes: usize,
}

/// Reads both identity areas, with an area that does not read as its own
/// answer so a page can say where.
///
/// # Errors
/// `E_STORAGE_FATAL` for a document that exists and cannot be read. A
/// document that is not there states nothing.
pub fn read_naming(city_root: &Path) -> Result<Result<Naming, Unreadable>, AxError> {
    let preferences = text_of(city_root, Governed::Preferences)?;
    let mayor = text_of(city_root, Governed::Mayor)?;
    Ok(Naming::of(&preferences, &mayor))
}

/// Rewrites one card's keys in `base` and lands the result whole, only if
/// the document still holds `base`.
///
/// # Errors
/// `E_CONFIG_INVALID` for a name outside [`DisplayName`]'s domain and for
/// a `base` whose identity area does not read; `E_VERSION_CONFLICT` when
/// the document is no longer `base`; `E_STORAGE_FATAL` when it cannot be
/// read or written.
pub fn write_naming(
    city_root: &Path,
    edit: &NamingEdit,
    base: &str,
) -> Result<NamingWritten, AxError> {
    let which = edit.document();
    let rewritten = rewrite(which, base, edit)?;
    crate::document::replace(&which.path(city_root), rewritten.as_bytes())?;
    Ok(NamingWritten {
        naming: Naming::read(city_root)?,
        bytes: rewritten.len(),
    })
}

/// Refuses a whole document whose identity area would not read, before it
/// is written: the raw editor's door holds the same reader as the cards.
///
/// # Errors
/// `E_CONFIG_INVALID` naming the line that does not read.
pub(crate) fn readable(which: Governed, body: &str) -> Result<(), AxError> {
    let area = split(which, body).map_err(Unreadable::into_ax)?;
    match which {
        Governed::Preferences => {
            name_key(&area, USER_ID).map_err(Unreadable::into_ax)?;
            name_key(&area, IMPORTED_FROM).map_err(Unreadable::into_ax)?;
        }
        Governed::Mayor => {
            name_key(&area, NAME).map_err(Unreadable::into_ax)?;
        }
        Governed::Clerk => {}
    }
    Ok(())
}

/// What a governed document says once its identity area is set aside:
/// the bytes that follow the closing fence, or the whole document when
/// it has no area.
#[must_use]
pub fn persona(written: Vec<u8>) -> Vec<u8> {
    let start = match std::str::from_utf8(&written) {
        Ok(text) => body_start(text),
        // Not text, so not an area either: what a person wrote is passed
        // on as it is.
        Err(_) => None,
    };
    match start.and_then(|start| written.get(start..)) {
        Some(body) => body.to_vec(),
        None => written,
    }
}

impl Naming {
    /// Both identity areas as they stand.
    ///
    /// # Errors
    /// Everything [`read_naming`] refuses, and an area that does not read
    /// as `E_CONFIG_INVALID` with its line.
    pub fn read(city_root: &Path) -> Result<Naming, AxError> {
        read_naming(city_root)?.map_err(Unreadable::into_ax)
    }

    fn of(preferences: &str, mayor: &str) -> Result<Naming, Unreadable> {
        let person = split(Governed::Preferences, preferences)?;
        let named = split(Governed::Mayor, mayor)?;
        Ok(Naming {
            person: name_key(&person, USER_ID)?,
            imported_from: name_key(&person, IMPORTED_FROM)?,
            about: person.body.trim().to_owned(),
            mayor: name_key(&named, NAME)?,
        })
    }

    #[must_use]
    pub fn person(&self) -> Option<&str> {
        self.person.as_ref().map(DisplayName::as_str)
    }

    #[must_use]
    pub fn imported_from(&self) -> Option<&str> {
        self.imported_from.as_ref().map(DisplayName::as_str)
    }

    #[must_use]
    pub fn about(&self) -> &str {
        &self.about
    }

    #[must_use]
    pub fn mayor(&self) -> Option<&str> {
        self.mayor.as_ref().map(DisplayName::as_str)
    }

    /// What the resident at `addr` is called: the Mayor's name at
    /// `hall/mayor` when one is set, and the last segment of the address
    /// everywhere else.
    #[must_use]
    pub fn called<'a>(&'a self, addr: &'a Address) -> &'a str {
        match (addr.as_str() == HALL_MAYOR, self.mayor()) {
            (true, Some(name)) => name,
            (true, None) | (false, _) => addr.name(),
        }
    }

    /// The bytes this naming is frozen under in the content store.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` if the names cannot be encoded, which a value
    /// of plain strings never is.
    pub fn to_bytes(&self) -> Result<Vec<u8>, AxError> {
        serde_json::to_vec(self).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "encode the identity a session freezes",
                err.to_string(),
            )
            .with_recovery("report this: a value of plain strings failed to encode")
        })
    }

    /// A naming read back from the bytes a session froze.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` for bytes that are not a frozen naming, which
    /// means the store holds something else under that version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Naming, AxError> {
        serde_json::from_slice(bytes).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the identity a session froze",
                err.to_string(),
            )
            .with_recovery("start a new session (`/new`), which freezes the names as they are now")
        })
    }

    /// The version a session freezes: the digest of [`Naming::to_bytes`].
    ///
    /// # Errors
    /// As [`Naming::to_bytes`].
    pub fn version(&self) -> Result<B3Hash, AxError> {
        Ok(B3Hash::digest(&self.to_bytes()?))
    }

    /// The block the city segment carries after `City.md`, or `None` when
    /// nothing is said, so a city that never named anyone sends the bytes
    /// it always sent.
    #[must_use]
    pub fn context(&self) -> Option<String> {
        if self.person.is_none() && self.mayor.is_none() && self.about.is_empty() {
            return None;
        }
        let mut out = String::from("## Who works here\n\n");
        if let Some(person) = self.person() {
            out.push_str(&format!(
                "The person this city works for is called {person}.\n"
            ));
        }
        if let Some(mayor) = self.mayor() {
            out.push_str(&format!("The Mayor, `{HALL_MAYOR}`, is called {mayor}.\n"));
        }
        if !self.about.is_empty() {
            out.push_str("\nWhat the person wants every agent here to know:\n\n");
            out.push_str(&self.about);
            out.push('\n');
        }
        out.push_str(
            "\nThese names come from the identity areas of PREFERENCES.md and MAYOR.md. A name \
             says what someone is called, not what they may do.\n",
        );
        Some(out)
    }
}

/// One document split at its identity area.
struct Split<'a> {
    document: Governed,
    /// The area's table, and the line its text starts on.
    area: Option<(toml::Table, &'a str)>,
    body: &'a str,
}

/// Splits a document at its identity area: a first line holding only
/// `+++`, TOML, and a line holding only `+++`.
fn split(document: Governed, text: &str) -> Result<Split<'_>, Unreadable> {
    let Some((opened, closed)) = fences(text) else {
        return Ok(Split {
            document,
            area: None,
            body: text,
        });
    };
    let unclosed = || Unreadable {
        document,
        line: 1,
        why: "the identity area opened here is never closed by a line holding only +++".to_owned(),
    };
    let closed = closed.ok_or_else(unclosed)?;
    let area_text = text.get(opened..closed.0).ok_or_else(unclosed)?;
    let body = text.get(closed.1..).ok_or_else(unclosed)?;
    let table = toml::from_str::<toml::Table>(area_text).map_err(|err| Unreadable {
        document,
        line: line_in(area_text, err.span().map_or(0, |span| span.start)),
        why: err.message().to_owned(),
    })?;
    Ok(Split {
        document,
        area: Some((table, area_text)),
        body,
    })
}

/// Where the area's text starts, and where the closing fence line starts
/// and ends; `None` when the first line is not a fence, and an inner
/// `None` when no line closes it.
fn fences(text: &str) -> Option<(usize, Option<(usize, usize)>)> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\n', '\r']) != FENCE {
        return None;
    }
    let mut at = first.len();
    for line in lines {
        let next = at.saturating_add(line.len());
        if line.trim_end_matches(['\n', '\r']) == FENCE {
            return Some((first.len(), Some((at, next))));
        }
        at = next;
    }
    Some((first.len(), None))
}

/// Where the body starts, by the fences alone: the area itself is not
/// read here.
fn body_start(text: &str) -> Option<usize> {
    fences(text)
        .and_then(|(_, closed)| closed)
        .map(|(_, end)| end)
}

/// The line of the document a byte of the area's text sits on, counting
/// the opening fence as line 1.
fn line_in(area_text: &str, at: usize) -> u32 {
    let before = area_text.get(..at).unwrap_or(area_text);
    let newlines = before.bytes().filter(|byte| *byte == b'\n').count();
    u32::try_from(newlines)
        .unwrap_or(u32::MAX)
        .saturating_add(2)
}

/// One name the area states, refused with the line it is on when it is
/// not a string or not a name.
fn name_key(split: &Split<'_>, key: &str) -> Result<Option<DisplayName>, Unreadable> {
    let Some((table, area_text)) = &split.area else {
        return Ok(None);
    };
    let refuse = |why: String| Unreadable {
        document: split.document,
        line: key_line(area_text, key),
        why,
    };
    match table.get(key) {
        None => Ok(None),
        Some(toml::Value::String(raw)) => DisplayName::parse(raw)
            .map(Some)
            .map_err(|err| refuse(format!("`{key}` {}", err.recovery()))),
        Some(other) => Err(refuse(format!(
            "`{key}` is a {}, and a name is a string",
            other.type_str()
        ))),
    }
}

/// The line `key = ...` sits on, or the opening fence when no line starts
/// with it (a key spelled inside an inline table, which TOML allows).
fn key_line(area_text: &str, key: &str) -> u32 {
    area_text
        .lines()
        .position(|line| {
            line.trim_start()
                .strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        })
        .map_or(1, |index| {
            u32::try_from(index).unwrap_or(u32::MAX).saturating_add(2)
        })
}

/// Writes one card's keys into `base`'s identity area and gives back the
/// whole document.
fn rewrite(which: Governed, base: &str, edit: &NamingEdit) -> Result<String, AxError> {
    let split = split(which, base).map_err(Unreadable::into_ax)?;
    let mut area = split.area.map(|(table, _)| table).unwrap_or_default();
    let body = match edit {
        NamingEdit::Person {
            user_id,
            imported_from,
            about,
        } => {
            set(&mut area, USER_ID, user_id.as_deref())?;
            set(&mut area, IMPORTED_FROM, imported_from.as_deref())?;
            about.as_deref().unwrap_or(split.body)
        }
        NamingEdit::Mayor { name } => {
            set(&mut area, NAME, name.as_deref())?;
            split.body
        }
    };
    compose(&area, body)
}

/// Writes one name into the area, or takes the key out.
fn set(area: &mut toml::Table, key: &str, value: Option<&str>) -> Result<(), AxError> {
    match value {
        Some(raw) => {
            let name = DisplayName::parse(raw)?;
            area.insert(key.to_owned(), toml::Value::String(name.0));
        }
        None => {
            area.remove(key);
        }
    }
    Ok(())
}

/// The whole document: the area between its fences, then the body. An
/// area with nothing left in it is dropped fences and all, so a name set
/// and then removed leaves the document as it was.
fn compose(area: &toml::Table, body: &str) -> Result<String, AxError> {
    if area.is_empty() {
        return Ok(body.to_owned());
    }
    let rendered = toml::to_string(area).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "write the identity area",
            err.to_string(),
        )
        .with_recovery("edit the identity area by hand in the raw editor")
    })?;
    Ok(format!("{FENCE}\n{rendered}{FENCE}\n{body}"))
}

/// A governed document's text, empty when it is not there.
fn text_of(city_root: &Path, which: Governed) -> Result<String, AxError> {
    let path = which.path(city_root);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the identity area",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix that file's permissions; the names in it are read at every dispatch")),
    }
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

    fn place(root: &Path, which: Governed, text: &str) {
        let path = which.path(root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A document written before identity areas existed is all body.
    #[test]
    fn a_document_without_an_identity_area_reads_as_body() {
        let dir = tempfile::tempdir().unwrap();
        place(dir.path(), Governed::Preferences, "I work in UTC.\n");
        let naming = Naming::read(dir.path()).unwrap();
        assert_eq!(naming.person(), None);
        assert_eq!(naming.about(), "I work in UTC.");
        assert_eq!(naming.mayor(), None);
    }

    /// Two writers started from one text; the second save is refused,
    /// and the file still holds what the first one wrote, which is what
    /// the second writer's page reads again before it tries once more.
    #[test]
    fn a_stale_identity_save_is_refused_and_the_draft_survives() {
        let dir = tempfile::tempdir().unwrap();
        place(dir.path(), Governed::Mayor, "Plans carefully.\n");
        let base = "Plans carefully.\n";
        write_naming(
            dir.path(),
            &NamingEdit::Mayor {
                name: Some("Cat".to_owned()),
            },
            base,
        )
        .unwrap();
        let landed = std::fs::read_to_string(Governed::Mayor.path(dir.path())).unwrap();

        let stale = write_naming(
            dir.path(),
            &NamingEdit::Mayor {
                name: Some("Dog".to_owned()),
            },
            base,
        );

        assert_eq!(
            stale.map_err(|err| *err.code()).err(),
            Some(AxCode::VersionConflict)
        );
        assert_eq!(
            std::fs::read_to_string(Governed::Mayor.path(dir.path())).unwrap(),
            landed
        );
        assert_eq!(Naming::read(dir.path()).unwrap().mayor(), Some("Cat"));
    }

    /// A name set and then taken away leaves the document byte for byte
    /// as it was, and a key the card does not own survives both writes.
    #[test]
    fn a_name_set_and_removed_leaves_the_document_as_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let original = "+++\ntheme = \"warm\"\n+++\nAbout me.\n";
        place(dir.path(), Governed::Preferences, original);
        let set = NamingEdit::Person {
            user_id: Some("2youg1".to_owned()),
            imported_from: Some("github.com".to_owned()),
            about: None,
        };
        let written = write_naming(dir.path(), &set, original).unwrap();
        assert_eq!(written.naming.person(), Some("2youg1"));
        assert_eq!(written.naming.about(), "About me.");
        let named = std::fs::read_to_string(Governed::Preferences.path(dir.path())).unwrap();
        assert!(named.contains("theme"), "{named}");

        let unset = NamingEdit::Person {
            user_id: None,
            imported_from: None,
            about: None,
        };
        write_naming(dir.path(), &unset, &named).unwrap();
        let back = std::fs::read_to_string(Governed::Preferences.path(dir.path())).unwrap();
        assert_eq!(back, "+++\ntheme = \"warm\"\n+++\nAbout me.\n");
    }

    /// An area that does not read is refused with the line it fails on,
    /// counted from the document's first line.
    #[test]
    fn an_identity_area_that_does_not_read_names_its_line() {
        let duplicated = "+++\nname = \"Cat\"\nname = \"Dog\"\n+++\nbody\n";
        let refused = split(Governed::Mayor, duplicated).err().unwrap();
        assert_eq!((refused.document, refused.line), (Governed::Mayor, 3));

        let unclosed = "+++\nname = \"Cat\"\nbody\n";
        let refused = split(Governed::Mayor, unclosed).err().unwrap();
        assert_eq!(refused.line, 1);

        let empty = "+++\n\nname = \"  \"\n+++\n";
        let refused = readable(Governed::Mayor, empty).unwrap_err();
        assert_eq!(*refused.code(), AxCode::ConfigInvalid);
        assert!(
            refused.subject().starts_with("MAYOR.md:3:"),
            "{}",
            refused.subject()
        );
    }

    /// What reaches the resident segment is what follows the area.
    #[test]
    fn the_identity_area_is_not_part_of_the_persona() {
        let written = b"+++\nname = \"Cat\"\n+++\nWho the Mayor is.\n".to_vec();
        assert_eq!(persona(written), b"Who the Mayor is.\n".to_vec());
        assert_eq!(persona(b"No area.\n".to_vec()), b"No area.\n".to_vec());
    }
}
