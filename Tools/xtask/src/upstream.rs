// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Require every row of `docs/third-party.md` to name one GitHub project, one
//! watched path and the commit that path was read at, because
//! `upstream-watch.yml` decomposes those cells itself: a cell holding two
//! paths is sent to the commits API as one path, answers nothing, and silently
//! ends the watch for every row after it.

use std::path::Path;

use crate::report::{Violation, XtaskError};

const DOC: &str = "docs/third-party.md";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let text = std::fs::read_to_string(root.join(DOC)).map_err(|source| XtaskError::Io {
        path: DOC.to_owned(),
        source,
    })?;
    Ok(judge(&text))
}

fn judge(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if !line.starts_with("| [") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let watched = line.contains("github.com/");
        let shaped = cells.len() == 7;
        if !watched && !shaped {
            continue;
        }
        if let Some(violation) = judge_row(index.saturating_add(1), &cells) {
            violations.push(violation);
        }
    }
    violations
}

fn judge_row(number: usize, cells: &[&str]) -> Option<Violation> {
    let [_, project, _licence, _followed, watched_cell, tracked_cell, _] = cells else {
        return Some(at(
            number,
            "a watched row has the six columns the table header names",
            format!(
                "this row splits into {} cells rather than six",
                cells.len().saturating_sub(2)
            ),
            "write the row as `| project | licence | what is followed | path | tracked to |`, one path",
        ));
    };
    if !github_project(project) {
        return Some(at(
            number,
            "a watched row names a GitHub project, since the watch reads the owner and the repository from the row",
            format!("`{project}` names no `github.com/<owner>/<repo>`"),
            "link the project as `https://github.com/<owner>/<repo>`",
        ));
    }
    let watched = paths(watched_cell);
    let [path] = watched.as_slice() else {
        return Some(at(
            number,
            "a watched row names one path, since the commits API is asked for that cell as one path",
            if watched.is_empty() {
                format!("`{watched_cell}` holds no backticked path")
            } else {
                format!("`{watched_cell}` holds {} paths", watched.len())
            },
            "give each followed path its own row, with the commit read for that path",
        ));
    };
    if path.contains(',') || path.contains(' ') {
        return Some(at(
            number,
            "a watched path is one path the commits API accepts",
            format!("`{path}` is not one path"),
            "split the row so each row carries one path, spelled as the upstream repository spells it",
        ));
    }
    let tracked = tracked_cell.trim_matches('`').trim();
    if tracked.len() < 12 || tracked.len() > 40 || !tracked.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Some(at(
            number,
            "a watched row records the commit its facts were read at, as 12 to 40 hexadecimal digits",
            format!("`{tracked_cell}` is not a commit"),
            "set `Tracked to` to the commit this path was read at, in the same change-set that reads it",
        ));
    }
    None
}

fn github_project(cell: &str) -> bool {
    let Some((_, rest)) = cell.split_once("github.com/") else {
        return false;
    };
    let repo = match rest.split(')').next() {
        Some(named) => named,
        None => return false,
    };
    let mut parts = repo.split('/');
    let (Some(owner), Some(name)) = (parts.next(), parts.next()) else {
        return false;
    };
    !owner.is_empty()
        && !name.is_empty()
        && !owner.contains(['(', ' '])
        && !name.contains(['(', ' ', '/'])
}

fn paths(cell: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = cell;
    while let Some((_, tail)) = rest.split_once('`') {
        let Some((path, after)) = tail.split_once('`') else {
            break;
        };
        found.push(path);
        rest = after;
    }
    found
}

fn at(number: usize, rule: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "upstream",
        location: format!("{DOC}:{number}"),
        rule: rule.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
