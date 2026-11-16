// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::path::Path;

use kernel::{AxCode, AxError, Payload};
use serde_json::{Map, Value};

use crate::tools::stamp::Stamp;

pub(super) fn as_the_file_spells<'a>(
    text: &str,
    old: &'a str,
    new: &'a str,
) -> (Cow<'a, str>, Cow<'a, str>) {
    if text.contains(old) {
        return (Cow::Borrowed(old), Cow::Borrowed(new));
    }
    let numbering = crate::rendering::Numbering::of(crate::rendering::CURRENT);
    let copied = crate::rendering::Copied::of(old, numbering);
    let mut confirmed = copied.starts().into_iter().filter_map(|start| {
        let (bare, numbers) = copied.drawn_from(start)?;
        begins_on_line(text, &bare, start).then_some((bare, numbers))
    });
    match (confirmed.next(), confirmed.next()) {
        (Some((bare, numbers)), None) => (
            Cow::Owned(bare),
            Cow::Owned(crate::rendering::Copied::of(new, numbering).without(&numbers)),
        ),
        (Some(_), Some(_)) | (None, _) => (Cow::Borrowed(old), Cow::Borrowed(new)),
    }
}

fn begins_on_line(text: &str, passage: &str, line: u64) -> bool {
    !passage.is_empty()
        && text.match_indices(passage).any(|(at, _)| {
            text.get(..at).is_some_and(|before| {
                (before.is_empty() || before.ends_with('\n'))
                    && u64::try_from(before.matches('\n').count()) == Ok(line)
            })
        })
}

pub(super) fn read_whole(rel: &str, path: &Path) -> Result<(String, Option<Stamp>), AxError> {
    let before = Stamp::of(path).map_err(|err| unreadable(rel, &err))?;
    if before.is_none() {
        return Ok((String::new(), None));
    }
    let bytes = std::fs::read(path).map_err(|err| unreadable(rel, &err))?;
    let after = Stamp::of(path).map_err(|err| unreadable(rel, &err))?;
    if before != after {
        return Err(AxError::failure(
            AxCode::VersionConflict,
            "edit file",
            format!("{rel} was being written while this edit read it"),
        )
        .with_recovery("read the file again, then edit"));
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "edit file",
            format!("{rel} is not valid UTF-8"),
        )
        .with_recovery(format!(
            "edit {rel} in a program that reads its bytes; this tool changes text \
             files and this one is not text"
        ))
    })?;
    Ok((text, after))
}

pub(super) fn unreadable(rel: &str, err: &std::io::Error) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "edit file", format!("{rel}: {err}"))
        .with_recovery("check the path; to create a new file pass old_string:\"\"")
}

pub(super) fn echo(rel: &str, before: &str, after: &str) -> Result<Payload, AxError> {
    let mut result = Map::new();
    result.insert("path".to_owned(), Value::String(rel.to_owned()));
    result.insert(
        "diff".to_owned(),
        Value::String(unified_diff(rel, before, after)),
    );
    Payload::new(result)
}

pub(super) fn refused_create(err: storage::StorageError, rel: &str) -> AxError {
    if !matches!(err, storage::StorageError::NameTaken { .. }) {
        return err.into_ax();
    }
    already_exists(rel)
}

pub(super) fn already_exists(rel: &str) -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "edit file",
        format!("{rel} already exists, so `old_string: \"\"` cannot create it"),
    )
    .with_recovery("read the file and replace a passage in it, or choose another path")
}

pub(super) fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let old_lines: Vec<&str> = before.lines().collect();
    let new_lines: Vec<&str> = after.lines().collect();
    let prefix = common_prefix(&old_lines, &new_lines);
    let suffix = common_suffix(&old_lines, &new_lines, prefix);
    let old_end = old_lines.len().saturating_sub(suffix);
    let new_end = new_lines.len().saturating_sub(suffix);

    let mut out = if before.is_empty() {
        format!("--- /dev/null\n+++ b/{path}\n")
    } else {
        format!("--- a/{path}\n+++ b/{path}\n")
    };
    let old_count = old_end.saturating_sub(prefix);
    let new_count = new_end.saturating_sub(prefix);
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        prefix.saturating_add(1),
        old_count,
        prefix.saturating_add(1),
        new_count
    ));
    for line in old_lines.get(prefix..old_end).unwrap_or_default() {
        out.push_str(&format!("-{line}\n"));
    }
    for line in new_lines.get(prefix..new_end).unwrap_or_default() {
        out.push_str(&format!("+{line}\n"));
    }
    out
}

pub(super) fn common_prefix(old: &[&str], new: &[&str]) -> usize {
    let mut i = 0usize;
    while let (Some(a), Some(b)) = (old.get(i), new.get(i)) {
        if a != b {
            break;
        }
        i = i.saturating_add(1);
    }
    i
}

pub(super) fn common_suffix(old: &[&str], new: &[&str], prefix: usize) -> usize {
    let mut i = 0usize;
    loop {
        let old_left = old.len().saturating_sub(i);
        let new_left = new.len().saturating_sub(i);
        if old_left <= prefix || new_left <= prefix {
            break;
        }
        let (Some(a), Some(b)) = (
            old.get(old_left.saturating_sub(1)),
            new.get(new_left.saturating_sub(1)),
        ) else {
            break;
        };
        if a != b {
            break;
        }
        i = i.saturating_add(1);
    }
    i
}
