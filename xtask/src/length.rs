// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Length gate: a body nobody reads to the end, and a file nobody can
//! navigate, both turn the build red.
//!
//! Two units, because they fail differently. A long **function** hides a
//! flow of control; a long **file** hides where anything is, and costs
//! every reader who has to find one thing in it.
//!
//! **The file rule has been re-priced twice, and each time the parameter
//! that moved is written beside it.** It began as a function rule only:
//! any honest file threshold lit up eight files at once, "which is a
//! project rather than a gate". The person asked for that project when
//! the largest file reached 12,078 lines, and 1000 lines was the line
//! that got it done - `bin::assembly` became an `assembly/` directory and
//! the register emptied. **A budget whose register is empty permits every
//! file in the tree**, which is what moved the line to 400 on
//! 2026-09-05: a file is now read whole into a model's context as often
//! as it is read by a person, and 400 lines is where a module still
//! arrives as one unit beside the other files a change must be read
//! against. Each re-pricing arrives with a register of the files that
//! predate it, pinned at the length they had on the day the line moved.
//!
//! **The register can only shrink, and it cleans itself.** A file it does
//! not name is refused at the budget outright, so the list cannot grow. A
//! file it does name may not exceed its pinned length, so no offender
//! grows. And a file that has come back under the budget must be struck
//! from the register, which turns the exception into something that
//! removes itself rather than something that has to be remembered.
//!
//! **The measurement parses.** Finding where a function begins and ends
//! by counting braces per line is wrong in three ways this repository
//! contains: `#[cfg(test)]` marks an item rather than the rest of a
//! file, `'{'` is a character and not a block, and a string may carry a
//! brace across a line continuation. Each mistake produces a wrong list
//! of offenders, and a gate that measures wrongly sends somebody to
//! break up a function that was never long. `syn` is the parser the
//! compiler's own macro ecosystem uses; it lands in workspace tooling
//! and never in the product binary.
//!
//! Two kinds are not measured, and each exemption is read from an
//! authority that already exists rather than from a list kept here:
//! an item marked `#[cfg(test)]`, and any file whose module-map row
//! states the shape `data` (ARCHITECTURE.md section 9, shape 6: data
//! with no branches).
//!
//! The `data` exemption covers the file rule as well as the function
//! rule, for the reason that granted it: a table is looked things up in
//! rather than navigated, so its length costs a reader nothing. That is
//! why `web::lang`, 3,217 lines of every word the client says in two
//! languages, is not in the register below.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::modmap;
use crate::report::{Violation, XtaskError};
use crate::walk;

mod measurement;

use measurement::{Found, measure};

/// Where first-party Rust lives. `tests/` and `benches/` are absent on
/// purpose: test code may relax what production code carries (AGENTS.md),
/// and a long test is a different question from a long function.
const SOURCE_DIRS: [&str; 3] = ["crates", "xtask/src", "citysim/src"];

/// The register row that states the limit, so the number lives with every
/// other budget the design states rather than inside this file.
const ROW: &str = "function_length";

/// The register row that states how long a file may be.
const FILE_ROW: &str = "file_length";

/// The sub-table naming the files that were already over the line when it
/// was drawn, each with the length it had that day.
const PREDATING: &str = "predating";

/// The register row that states how many parameters a function may take.
const ARG_ROW: &str = "argument_count";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let body_limit = limit(root, ROW)?;
    let file_limit = limit(root, FILE_ROW)?;
    let arg_limit = limit(root, ARG_ROW)?;
    let excused = excused(root)?;
    let predating = predating(root)?;
    let shapes = modmap::shapes(root)?;
    let mut violations = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for file in sources(root)? {
        let rel = walk::rel(root, &file);
        if shapes.get(&rel).is_some_and(|shape| shape == "data") {
            continue;
        }
        let text = walk::read_text(&file)?;
        let lines = text.lines().count();
        seen.insert(rel.clone());
        match predating.get(&rel) {
            None => {
                if lines > file_limit {
                    violations.push(too_long(&rel, lines, file_limit));
                }
            }
            Some(&pinned) => {
                if lines > pinned {
                    violations.push(grew(&rel, lines, pinned));
                } else if lines <= file_limit {
                    violations.push(no_longer_an_exception(&rel, lines, file_limit));
                }
            }
        }
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        for found in measure(&parsed.items) {
            if found.lines > body_limit {
                violations.push(over(&rel, &found, body_limit));
            }
            if found.args > arg_limit && !excused.contains(&key(&rel, &found.name)) {
                violations.push(too_many_arguments(&rel, &found, arg_limit));
            }
        }
    }
    // A row naming a file that is no longer measured - renamed, split
    // away, or deleted - is a pin nothing holds. Left alone it would
    // silently re-admit that path if anybody ever recreated it.
    for stale in predating.keys().filter(|rel| !seen.contains(*rel)) {
        violations.push(Violation {
            gate: "length",
            location: format!("xtask/budgets.toml [{FILE_ROW}.{PREDATING}]"),
            rule: "every file the register pins is a file this gate measures".to_owned(),
            violation: format!("{stale} is pinned and no longer here"),
            alternative: "delete the row: the debt it recorded has been paid or moved".to_owned(),
        });
    }
    Ok(violations)
}

fn too_long(rel: &str, lines: usize, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: rel.to_owned(),
        rule: format!("a source file stays inside {limit} lines (xtask/budgets.toml, {FILE_ROW})"),
        violation: format!("{lines} lines"),
        alternative: "give each responsibility in it its own file and name, and let this one \
                      keep the part that routes between them"
            .to_owned(),
    }
}

fn grew(rel: &str, lines: usize, pinned: usize) -> Violation {
    Violation {
        gate: "length",
        location: rel.to_owned(),
        rule: format!(
            "a file the register pins may only get smaller \
             (xtask/budgets.toml, {FILE_ROW}.{PREDATING})"
        ),
        violation: format!("{lines} lines, pinned at {pinned}"),
        alternative: "put the new code in a file of its own: this one is already over the \
                      budget and is waiting to be split"
            .to_owned(),
    }
}

fn no_longer_an_exception(rel: &str, lines: usize, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("xtask/budgets.toml [{FILE_ROW}.{PREDATING}]"),
        rule: "an exception that is no longer needed is struck from the register".to_owned(),
        violation: format!("{rel} is {lines} lines, inside the {limit} the rule states"),
        alternative: "delete its row: the split is done, and a spent exception left in place \
                      is a licence nobody decided to keep granting"
            .to_owned(),
    }
}

/// The files that were already over the line when it was drawn, each
/// pinned at the length it had that day.
fn predating(root: &Path) -> Result<BTreeMap<String, usize>, XtaskError> {
    let register = crate::budget::register(root)?;
    let Some(table) = register
        .get(FILE_ROW)
        .and_then(|row| row.get(PREDATING))
        .and_then(toml::Value::as_table)
    else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for (rel, value) in table {
        let stated = value.as_integer().ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("{FILE_ROW}.{PREDATING}.{rel} is not a line count"),
        })?;
        let pinned = usize::try_from(stated).map_err(|_| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("{FILE_ROW}.{PREDATING}.{rel} is not a line count: {stated}"),
        })?;
        out.insert(rel.clone(), pinned);
    }
    Ok(out)
}

fn too_many_arguments(rel: &str, found: &Found, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("{rel}:{}", found.line),
        rule: format!(
            "a function takes at most {limit} parameters \
             (xtask/budgets.toml, {ARG_ROW})"
        ),
        violation: format!("{} takes {}", found.name, found.args),
        alternative: "the ones that always travel together are one value: give them a struct \
                      with a name, as `Reporter` did for the four that describe who is \
                      reporting a change to a plan"
            .to_owned(),
    }
}

fn over(rel: &str, found: &Found, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("{rel}:{}", found.line),
        rule: format!(
            "a production function stays inside {limit} lines \
             (xtask/budgets.toml, function_length)"
        ),
        violation: format!("{} is {} lines", found.name, found.lines),
        alternative: "give a phase of it its own name, and hand the values it produces back \
                      as one value"
            .to_owned(),
    }
}

fn limit(root: &Path, row: &str) -> Result<usize, XtaskError> {
    let register = crate::budget::register(root)?;
    let stated = register
        .get(row)
        .and_then(|found| {
            found
                .get("budget_lines")
                .or_else(|| found.get("budget_arguments"))
        })
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("{row} states no budget_lines"),
        })?;
    usize::try_from(stated).map_err(|_| XtaskError::Doc {
        file: "xtask/budgets.toml".to_owned(),
        msg: format!("{row}.budget_lines is not a line count: {stated}"),
    })
}

fn sources(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let mut out = Vec::new();
    for dir in SOURCE_DIRS {
        let base = root.join(dir);
        if !base.exists() {
            continue;
        }
        for file in walk::files_with_ext(&base, &["rs"])? {
            // Only a crate's own sources; `crates/*/tests` and the fuzz
            // targets are test code by another name.
            let rel = walk::rel(root, &file);
            if rel.starts_with("crates/") && !rel.contains("/src/") {
                continue;
            }
            out.push(file);
        }
    }
    Ok(out)
}

/// How a function is named in the register: the file it lives in and its
/// own name. Two functions in one file cannot share a name, and a name
/// alone would excuse every `new` in the workspace at once.
fn key(rel: &str, name: &str) -> String {
    format!("{rel}::{name}")
}

/// The functions whose parameter lists predate the rule.
fn excused(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let register = crate::budget::register(root)?;
    let Some(listed) = register
        .get(ARG_ROW)
        .and_then(|row| row.get(PREDATING))
        .and_then(|table| table.get("names"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(BTreeSet::new());
    };
    let mut out = BTreeSet::new();
    for value in listed {
        let named = value.as_str().ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("{ARG_ROW}.{PREDATING} holds something that is not a name"),
        })?;
        out.insert(named.to_owned());
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
