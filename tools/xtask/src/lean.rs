// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A Lean file read as text, in the few restricted shapes a gate reads
//! (xtask-SPEC.md sections 8-42 and 8-43).
//!
//! **No gate runs Lean** (section 12-8). A gate judges in milliseconds
//! beside twenty others, and proving belongs to `just models`; what a
//! gate needs from a specification is a roster, a table or an import
//! line, and those are written one per line so that reading them takes a
//! few lines here. Lean holds the other half: a `def` whose match leaves
//! out a constructor does not build, so a gate only asks whether the two
//! sides name the same things.
//!
//! **One spelling on both sides.** A Lean constructor carries the Rust
//! variant's name exactly, so nothing here converts case or inserts
//! underscores; a conversion rule would be a second grammar.

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

/// The entry of a crate's specification, in its package directory. A
/// package has migrated to Lean exactly when this file is there.
pub(crate) const ENTRY: &str = "Spec.lean";

/// The directory, beside the entry, that holds its parts.
pub(crate) const PARTS: &str = "spec";

/// One Lean file: where it is, repo-relative, and what it says.
pub(crate) struct Source {
    pub(crate) path: String,
    pub(crate) text: String,
}

/// Whether the package at `dir` keeps its specification in Lean.
#[must_use]
pub(crate) fn migrated(root: &Path, dir: &str) -> bool {
    root.join(dir).join(ENTRY).is_file()
}

/// The Lean specification of the package at `dir`: its entry and every
/// part under `spec/`, or `None` while the package has not migrated.
///
/// # Errors
/// When a file of it cannot be read.
pub(crate) fn specification(root: &Path, dir: &str) -> Result<Option<Vec<Source>>, XtaskError> {
    if !migrated(root, dir) {
        return Ok(None);
    }
    let parts = root.join(dir).join(PARTS);
    let mut files = vec![root.join(dir).join(ENTRY)];
    if parts.is_dir() {
        files.extend(walk::files_with_ext(&parts, &["lean"])?);
    }
    files
        .iter()
        .map(|file| {
            Ok(Source {
                path: walk::rel(root, file),
                text: walk::read_text(file)?,
            })
        })
        .collect::<Result<Vec<_>, XtaskError>>()
        .map(Some)
}

/// `text` with every comment and string blanked to spaces. Newlines stay
/// where they were, so a line number read from the result is the line
/// number of the file.
///
/// A comment is `--` to the end of the line or `/-` to its matching
/// `-/`, and block comments nest, as Lean's do. A character literal of a
/// double quote would open a string here; no specification writes one.
#[must_use]
pub(crate) fn code(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut mode = Mode::Code;
    let mut index = 0usize;
    while let Some(&current) = chars.get(index) {
        let next = chars.get(index.saturating_add(1)).copied();
        let (width, keep) = step(&mut mode, current, next);
        for offset in 0..width {
            if let Some(&character) = chars.get(index.saturating_add(offset)) {
                out.push(if keep || character == '\n' {
                    character
                } else {
                    ' '
                });
            }
        }
        index = index.saturating_add(width);
    }
    out
}

/// What the reader is inside when it reaches a character.
enum Mode {
    Code,
    Line,
    Block(usize),
    Text,
}

/// How many characters the reader takes at `current`, and whether they
/// are code; moves `mode` past them.
fn step(mode: &mut Mode, current: char, next: Option<char>) -> (usize, bool) {
    match mode {
        Mode::Code => match (current, next) {
            ('-', Some('-')) => {
                *mode = Mode::Line;
                (2, false)
            }
            ('/', Some('-')) => {
                *mode = Mode::Block(1);
                (2, false)
            }
            ('"', _) => {
                *mode = Mode::Text;
                (1, false)
            }
            _ => (1, true),
        },
        Mode::Line => {
            if current == '\n' {
                *mode = Mode::Code;
            }
            (1, false)
        }
        Mode::Block(depth) => match (current, next) {
            ('/', Some('-')) => {
                *depth = depth.saturating_add(1);
                (2, false)
            }
            ('-', Some('/')) => {
                let left = depth.saturating_sub(1);
                *mode = if left == 0 {
                    Mode::Code
                } else {
                    Mode::Block(left)
                };
                (2, false)
            }
            _ => (1, false),
        },
        Mode::Text => match current {
            '\\' => (2, false),
            '"' => {
                *mode = Mode::Code;
                (1, false)
            }
            _ => (1, false),
        },
    }
}

/// The constructors an `inductive` declares, one per line.
pub(crate) struct Roster {
    /// The 1-based line of `inductive`.
    pub(crate) line: usize,
    pub(crate) names: Vec<String>,
}

/// One arm of a `def` written as a table: `| .Pattern => value`.
pub(crate) struct Arm {
    pub(crate) line: usize,
    pub(crate) pattern: String,
    /// The right-hand side with its whitespace squeezed to single spaces.
    pub(crate) value: String,
}

/// A `def` written one arm per line.
pub(crate) struct Table {
    pub(crate) line: usize,
    pub(crate) arms: Vec<Arm>,
}

/// Every `inductive <Name> where` in `code` (already blanked by [`code`]),
/// by name, with the constructor on each `|` line below it.
#[must_use]
pub(crate) fn inductives(code: &str) -> BTreeMap<String, Roster> {
    let lines: Vec<&str> = code.lines().collect();
    let mut out = BTreeMap::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("inductive ") else {
            continue;
        };
        let name = ident(rest);
        if name.is_empty() {
            continue;
        }
        let names = block(&lines, index)
            .into_iter()
            .map(|(_, row)| ident(row.trim_start_matches('.')))
            .filter(|found| !found.is_empty())
            .collect();
        out.insert(
            name,
            Roster {
                line: index.saturating_add(1),
                names,
            },
        );
    }
    out
}

/// The arms of `def <name>` in `code`, or `None` when no such `def` is
/// there.
#[must_use]
pub(crate) fn arms(code: &str, name: &str) -> Option<Table> {
    let lines: Vec<&str> = code.lines().collect();
    let head = format!("def {name}");
    let index = lines.iter().position(|line| {
        line.trim_start()
            .strip_prefix(head.as_str())
            .is_some_and(|tail| tail.starts_with([' ', ':']))
    })?;
    let arms = block(&lines, index)
        .into_iter()
        .map(|(line, row)| {
            let (pattern, value) = row.split_once("=>").unwrap_or((row, ""));
            Arm {
                line,
                pattern: ident(pattern.trim_start().trim_start_matches('.')),
                value: value.split_whitespace().collect::<Vec<_>>().join(" "),
            }
        })
        .collect();
    Some(Table {
        line: index.saturating_add(1),
        arms,
    })
}

/// Every module `code` imports, with the 1-based line of its `import`.
#[must_use]
pub(crate) fn imports(code: &str) -> Vec<(usize, String)> {
    code.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let rest = line.trim_start().strip_prefix("import ")?;
            Some(
                rest.split_whitespace()
                    .map(move |module| (index.saturating_add(1), module.to_owned())),
            )
        })
        .flatten()
        .collect()
}

/// The repo-relative file a module name is read from:
/// `crates.storage.spec.Snapshot` is `crates/storage/spec/Snapshot.lean`.
#[must_use]
pub(crate) fn module_file(module: &str) -> String {
    format!("{}.lean", module.replace('.', "/"))
}

/// A package directory in the dotted form a module name starts with:
/// `crates/agent_protocols` is `crates.agent_protocols`.
#[must_use]
pub(crate) fn dotted(dir: &str) -> String {
    dir.replace('/', ".")
}

/// The rows of the one-per-line block below line `index`, each after its
/// `|`, with its 1-based line. Blank lines, which is what a comment
/// becomes, are skipped; the first other line that does not open with
/// `|` ends the block.
fn block<'a>(lines: &[&'a str], index: usize) -> Vec<(usize, &'a str)> {
    lines
        .iter()
        .enumerate()
        .skip(index.saturating_add(1))
        .filter(|(_, line)| !line.trim().is_empty())
        .map_while(|(at, line)| {
            line.trim()
                .strip_prefix('|')
                .map(|row| (at.saturating_add(1), row.trim()))
        })
        .collect()
}

/// The identifier `text` opens with; a dotted name keeps its dots.
fn ident(text: &str) -> String {
    text.trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '\'' | '.'))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
