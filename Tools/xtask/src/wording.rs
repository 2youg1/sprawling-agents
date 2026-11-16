// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reader-facing words come from `lang.json`. `markup` reads Svelte
//! text nodes and spoken attributes; `refusal` reads the corresponding
//! TypeScript positions. Generated files are judged at their generator.
//! Interpolations are removed before checking for adjacent letters.
//! `wording-ok` covers an indentation block so formatting cannot move
//! its scope to a different line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The only client source root; wire values and error codes are not reader prose.
use crate::walk::CLIENT_SRC as CLIENT;

/// The waiver, spelled as `lexicon`'s is.
const EXEMPT_MARK: &str = "wording-ok:";

/// HTML and ARIA attributes that carry author-supplied reader text.
const SPOKEN: [&str; 8] = [
    "placeholder",
    "title",
    "alt",
    "aria-label",
    "aria-description",
    "aria-placeholder",
    "aria-roledescription",
    "aria-valuetext",
];

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let dir = root.join(CLIENT);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let phrases = keys(root)?;
    let mut violations = Vec::new();
    let markup = Rule {
        read: markup::handed_to_a_reader,
        phrases: &phrases,
    };
    for path in walk::files_with_ext(&dir, &["svelte"])? {
        judge(root, &path, &markup, &mut violations)?;
    }
    let modules = Rule {
        read: refusal::handed_to_a_reader,
        phrases: &phrases,
    };
    for path in walk::files_with_ext(&dir, &["ts"])? {
        judge(root, &path, &modules, &mut violations)?;
    }
    Ok(violations)
}

/// Every phrase key defined by `lang.json`.
fn keys(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let path = root.join(CLIENT).join("lang.json");
    let text = walk::read_text(&path)?;
    let table: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&text).map_err(|err| XtaskError::Doc {
            file: walk::rel(root, &path),
            msg: format!("this file does not parse as JSON: {err}"),
        })?;
    Ok(table.into_keys().collect())
}

/// One language reader and the phrase keys shared by every file in a run.
struct Rule<'a> {
    read: fn(&str) -> Vec<Said>,
    phrases: &'a BTreeSet<String>,
}

/// Holds one file to the rule, through the reader its language needs.
fn judge(
    root: &Path,
    path: &Path,
    rule: &Rule<'_>,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let location = walk::rel(root, path);
    if location.contains(".test.") {
        return Ok(());
    }
    let text = walk::read_text(path)?;
    if walk::generated(&text) {
        return Ok(());
    }
    let lines: Vec<&str> = text.lines().collect();
    for said in (rule.read)(drawn(&text)) {
        if waived(&lines, said.line) || rule.phrases.contains(&said.left) {
            continue;
        }
        violations.push(Violation {
            gate: "wording",
            location: format!("{location}:{}", said.line),
            rule: "a word a reader is given comes from lang.json, not from the view".to_owned(),
            violation: format!(
                "{} carries {:?}, which no phrase produced",
                said.seat,
                clipped(&said.left)
            ),
            alternative:
                "add a Msg with both languages and fill its named slots; if this text is not reader-facing, narrow the parser to that exact context instead of adding a waiver"
                    .to_owned(),
        });
    }
    Ok(())
}

/// The production portion preceding a module's test items.
fn drawn(body: &str) -> &str {
    match body.find("#[cfg(test)]") {
        Some(at) => body.get(..at).unwrap_or(body),
        None => body,
    }
}

/// Whether an indentation-scoped waiver covers `line`.
fn waived(lines: &[&str], line: usize) -> bool {
    let mut floor = usize::MAX;
    for text in lines.iter().take(line).rev() {
        if text.trim().is_empty() {
            continue;
        }
        let depth = indent(text);
        if text.contains(EXEMPT_MARK) && floor >= depth {
            return true;
        }
        floor = floor.min(depth);
    }
    false
}

/// How far a line is indented, in bytes of leading whitespace.
fn indent(text: &str) -> usize {
    text.len().saturating_sub(text.trim_start().len())
}

fn clipped(text: &str) -> String {
    let mut out: String = text.chars().take(60).collect();
    if out.chars().count() < text.chars().count() {
        out.push('…');
    }
    out
}

/// One literal a reader is handed, and the words left in it.
struct Said {
    line: usize,
    seat: &'static str,
    left: String,
}

mod markup;
mod refusal;

/// Text remaining after interpolations and escapes, when it contains adjacent letters.
fn words_the_view_wrote(literal: &str) -> Option<String> {
    let mut left = String::new();
    let mut chars = literal.chars().peekable();
    let mut depth = 0_usize;
    while let Some(here) = chars.next() {
        match here {
            '\\' => {
                if chars.next() == Some('u') && chars.peek() == Some(&'{') {
                    for inner in chars.by_ref() {
                        if inner == '}' {
                            break;
                        }
                    }
                }
            }
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
            }
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => left.push(here),
            _ => {}
        }
    }
    let mut run = 0_usize;
    for here in left.chars() {
        run = if here.is_ascii_alphabetic() {
            run.saturating_add(1)
        } else {
            0
        };
        if run >= 2 {
            return Some(left.trim().to_owned());
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests;
