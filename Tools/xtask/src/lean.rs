// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

pub(crate) const ENTRY: &str = "Spec.lean";

pub(crate) const PARTS: &str = "spec";

pub(crate) struct Source {
    pub(crate) path: String,
    pub(crate) text: String,
}

#[must_use]
pub(crate) fn migrated(root: &Path, dir: &str) -> bool {
    root.join(dir).join(ENTRY).is_file()
}

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

enum Mode {
    Code,
    Line,
    Block(usize),
    Text,
}

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

pub(crate) struct Roster {

    pub(crate) line: usize,
    pub(crate) names: Vec<String>,
}

pub(crate) struct Arm {
    pub(crate) line: usize,
    pub(crate) pattern: String,

    pub(crate) value: String,
}

pub(crate) struct Table {
    pub(crate) line: usize,
    pub(crate) arms: Vec<Arm>,
}

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

#[must_use]
pub(crate) fn module_file(module: &str) -> String {
    format!("{}.lean", module.replace('.', "/"))
}

#[must_use]
pub(crate) fn dotted(dir: &str) -> String {
    dir.replace('/', ".")
}

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

fn ident(text: &str) -> String {
    text.trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '\'' | '.'))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
