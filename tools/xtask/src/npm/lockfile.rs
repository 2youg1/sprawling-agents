// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How the three data faces of the `npm` gate are read: the client's
//! manifest, the lockfile bun solved from it, and the one licence
//! allowlist this repository keeps.
//!
//! `bun.lock` is JSONC — trailing commas, and comments where bun wants
//! to explain itself — so it is normalised before `serde_json` sees it.
//! Normalising rather than hand-parsing is what keeps one reader for
//! both files: the manifest and the lockfile then arrive in the same
//! shape and can be compared key by key.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::report::XtaskError;

/// The two dependency tables a manifest or a lockfile states, in the one
/// shape both are compared in.
///
/// The two always travel together — every assertion in this gate reads
/// one against the other — so they are one named value rather than two
/// maps a caller has to keep paired.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Manifest {
    pub(super) runtime: BTreeMap<String, String>,
    pub(super) development: BTreeMap<String, String>,
}

/// JSONC to JSON: comments out, trailing commas out, strings untouched.
///
/// The string state is tracked because a `//` inside a URL and a `,` in
/// a version range are content, and a normaliser that edited them would
/// silently change what the gate then compares.
pub(super) fn read_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(here) = chars.next() {
        if in_string {
            out.push(here);
            match here {
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match here {
            '"' => {
                in_string = true;
                out.push(here);
            }
            '/' if chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = '\0';
                for skipped in chars.by_ref() {
                    if previous == '*' && skipped == '/' {
                        break;
                    }
                    previous = skipped;
                }
            }
            ',' => {
                // A comma is trailing when the next thing that is not
                // whitespace closes the collection. The whitespace it
                // stepped over is copied back either way, so no line
                // number in a parse error moves.
                let (closing, skipped) = closes_next(&mut chars);
                if !closing {
                    out.push(here);
                }
                out.push_str(&skipped);
            }
            _ => out.push(here),
        }
    }
    out
}

/// Whether the next non-blank character closes a collection, and the
/// whitespace stepped over on the way to it.
fn closes_next(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> (bool, String) {
    let mut skipped = String::new();
    while let Some(next) = chars.peek() {
        if next.is_whitespace() {
            skipped.push(*next);
            chars.next();
            continue;
        }
        return (*next == '}' || *next == ']', skipped);
    }
    (false, skipped)
}

/// Reads one JSONC document, naming the file when it will not parse.
///
/// # Errors
/// Reports a document this gate cannot read at all, which is a broken
/// gate rather than a violation: judging nothing is not the same fact as
/// finding nothing.
pub(super) fn document(file: &str, text: &str) -> Result<Value, XtaskError> {
    serde_json::from_str(&read_jsonc(text)).map_err(|err| XtaskError::Doc {
        file: file.to_owned(),
        msg: format!("cannot be read as JSON: {err}"),
    })
}

/// The two tables `client/package.json` states.
pub(super) fn manifest_of(document: &Value) -> Manifest {
    Manifest {
        runtime: table(document.get("dependencies")),
        development: table(document.get("devDependencies")),
    }
}

/// The two tables `client/bun.lock` recorded for the root workspace.
///
/// # Errors
/// Refuses a lockfile whose shape this version does not read, because a
/// missing `workspaces` block would otherwise compare as two empty
/// tables and report the whole manifest as drift.
pub(super) fn lock_of(document: &Value) -> Result<Manifest, XtaskError> {
    let root = document
        .get("workspaces")
        .and_then(|held| held.get(""))
        .ok_or_else(|| XtaskError::Doc {
            file: "client/bun.lock".to_owned(),
            msg: "no `workspaces` entry for the root package".to_owned(),
        })?;
    Ok(manifest_of(root))
}

/// One JSON object read as a string table; anything that is not a string
/// is dropped, since a dependency whose value is an object is a shape
/// this gate does not compare.
fn table(value: Option<&Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(Value::Object(entries)) = value else {
        return out;
    };
    for (name, held) in entries {
        if let Value::String(range) = held {
            out.insert(name.clone(), range.clone());
        }
    }
    out
}

/// The licences this repository permits, from the one place it says so.
///
/// `deny.toml` is the authority for the workspace side already, and a
/// repository has one position on licences rather than two.
///
/// # Errors
/// Refuses a `deny.toml` with no allowlist: an empty set would report
/// every package in the tree, which is a broken reader wearing the
/// costume of a finding.
pub(super) fn permitted(text: &str) -> Result<BTreeSet<String>, XtaskError> {
    let parsed: toml::Value = toml::from_str(text).map_err(|err| XtaskError::Doc {
        file: "deny.toml".to_owned(),
        msg: format!("cannot be read as TOML: {err}"),
    })?;
    let allowed = parsed
        .get("licenses")
        .and_then(|held| held.get("allow"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| XtaskError::Doc {
            file: "deny.toml".to_owned(),
            msg: "no `[licenses] allow` list".to_owned(),
        })?;
    let out: BTreeSet<String> = allowed
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect();
    if out.is_empty() {
        return Err(XtaskError::Doc {
            file: "deny.toml".to_owned(),
            msg: "the `[licenses] allow` list is empty".to_owned(),
        });
    }
    Ok(out)
}

/// Whether an SPDX expression names a licence this repository permits.
///
/// An `OR` offers a choice, so one permitted branch is enough; an `AND`
/// imposes all of them, so every branch must be permitted. This is how
/// `cargo-deny` reads the same expression, and two readers that
/// disagreed would let one side of the tree carry what the other bans.
pub(super) fn allowed(expression: &str, permitted: &BTreeSet<String>) -> bool {
    let cleaned = expression.replace(['(', ')'], " ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains(" OR ") {
        return trimmed
            .split(" OR ")
            .any(|branch| allowed(branch, permitted));
    }
    if trimmed.contains(" AND ") {
        return trimmed
            .split(" AND ")
            .all(|branch| allowed(branch, permitted));
    }
    permitted.contains(trimmed)
}
