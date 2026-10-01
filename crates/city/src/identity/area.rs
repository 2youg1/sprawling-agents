// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The identity area's grammar (city-SPEC.md 8-33): where it starts and
//! ends, the names it states and the line each one sits on, and how one
//! card's keys are written back into it.

use kernel::{AxCode, AxError};

use super::{DisplayName, NamingEdit, Unreadable};
use crate::governed::Governed;

/// The line that opens and closes an identity area.
pub(super) const FENCE: &str = "+++";
pub(super) const USER_ID: &str = "user_id";
pub(super) const IMPORTED_FROM: &str = "imported_from";
pub(super) const NAME: &str = "name";

/// One document split at its identity area.
pub(super) struct Split<'a> {
    pub(super) document: Governed,
    /// The area's table, and the line its text starts on.
    pub(super) area: Option<(toml::Table, &'a str)>,
    pub(super) body: &'a str,
}

/// Splits a document at its identity area: a first line holding only
/// `+++`, TOML, and a line holding only `+++`.
pub(super) fn split(document: Governed, text: &str) -> Result<Split<'_>, Unreadable> {
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
pub(super) fn body_start(text: &str) -> Option<usize> {
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
pub(super) fn name_key(split: &Split<'_>, key: &str) -> Result<Option<DisplayName>, Unreadable> {
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
pub(super) fn rewrite(which: Governed, base: &str, edit: &NamingEdit) -> Result<String, AxError> {
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
