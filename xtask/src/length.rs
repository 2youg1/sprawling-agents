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
//!
//! **The file rule reaches the client and the out-of-tree desktop
//! package.** Neither is compiled by a workspace command, and a rule
//! whose only executor is a compiler is a rule that stops at the wall.
//!
//! **The file rule reaches the client, and only the file rule.** The
//! client is 19,000 lines of TypeScript read by the same two readers,
//! and until this gate reached it the 400-line line had no executor
//! there at all. What does not reach it is the function rule: measuring
//! a function means parsing the language it is written in, `syn` parses
//! Rust, and no TypeScript parser may be added to this workspace's
//! manifest for a gate. A gate that guessed at a body's bounds by
//! counting braces is the mistake this module's own history records
//! three times, so the client's function lengths stay unmeasured and
//! said so, rather than measured wrongly.
//!
//! A generated file is not measured on either side. `client/src/wire.ts`
//! is 2,445 lines that `cargo xtask wire-ts` writes from the Rust wire;
//! splitting it would be splitting the generator's output, and the
//! exemption is read from the banner the generator itself writes rather
//! than from a path kept here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::modmap;
use crate::report::{Violation, XtaskError};
use crate::walk;

mod measurement;
mod register;

use measurement::{Found, measure};
use register::{ARG_ROW, FILE_ROW, PREDATING, ROW, excused, key, limit, predating};

/// Where first-party Rust lives. `tests/` and `benches/` are absent on
/// purpose: test code may relax what production code carries (AGENTS.md),
/// and a long test is a different question from a long function.
///
/// **`desktop/src` is here although it is not in the workspace.** That
/// package is compiled by no workspace command (root `Cargo.toml`,
/// `exclude`), and the rule it states for itself is this one
/// (desktop-SPEC.md section 16). A gate reads files rather than crates,
/// so it reaches where `cargo` does not — and the same 400 lines apply,
/// because the reader who has to find one thing in a long file is the
/// same reader on both sides of that wall.
const SOURCE_DIRS: [&str; 4] = ["crates", "xtask/src", "citysim/src", "desktop/src"];

/// Where the client's own sources live. Only the file rule reaches
/// them; the module documentation says why. The directory itself is
/// `walk`'s to state - five gates used to hold their own spelling of
/// it, and nothing compared them.
use crate::walk::CLIENT_SRC as CLIENT_DIR;

/// What the client is written in.
const CLIENT_EXTENSIONS: [&str; 2] = ["ts", "tsx"];

/// What a generator writes above the file it produced. A file carrying
/// it is the generator's output rather than somebody's module, so its
/// length is a fact about the generator.
const GENERATED: &str = "Generated by `cargo xtask";

/// The file rule as the register states it: the budget every file is
/// held to, and the pins of the files that predate it. The two are read
/// together and judged together, so they travel as one value.
struct FileRule {
    limit: usize,
    predating: BTreeMap<String, usize>,
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let body_limit = limit(root, ROW)?;
    let arg_limit = limit(root, ARG_ROW)?;
    let excused = excused(root)?;
    let files = FileRule {
        limit: limit(root, FILE_ROW)?,
        predating: predating(root)?,
    };
    let shapes = modmap::shapes(root)?;
    let mut violations = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    // How wide each excused signature is today. Only the excused ones
    // are recorded, because the register is the only reason to remember
    // a signature that is inside the budget.
    //
    // The widest, because one address can hold more than one function:
    // `workshop.rs::new` names an eight-parameter constructor and a
    // one-parameter one, and keeping whichever was parsed last reported
    // a live exception as spent.
    let mut still_here: BTreeMap<String, usize> = BTreeMap::new();
    for file in sources(root)? {
        let rel = walk::rel(root, &file);
        if shapes.get(&rel).is_some_and(|shape| shape == "data") {
            continue;
        }
        let text = walk::read_text(&file)?;
        seen.insert(rel.clone());
        judge_file(&files, &rel, &text, &mut violations);
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        for found in measure(&parsed.items) {
            if found.lines > body_limit {
                violations.push(over(&rel, &found, body_limit));
            }
            let named = key(&rel, &found.name);
            if excused.contains(&named) {
                let widest = still_here.entry(named).or_insert(found.args);
                *widest = (*widest).max(found.args);
            } else if found.args > arg_limit {
                violations.push(too_many_arguments(&rel, &found, arg_limit));
            }
        }
    }
    for file in client_sources(root)? {
        let rel = walk::rel(root, &file);
        let text = walk::read_text(&file)?;
        if generated(&text) {
            continue;
        }
        seen.insert(rel.clone());
        judge_file(&files, &rel, &text, &mut violations);
    }
    // A row naming a file that is no longer measured - renamed, split
    // away, or deleted - is a pin nothing holds. Left alone it would
    // silently re-admit that path if anybody ever recreated it.
    for stale in files.predating.keys().filter(|rel| !seen.contains(*rel)) {
        violations.push(Violation {
            gate: "length",
            location: format!("xtask/budgets.toml [{FILE_ROW}.{PREDATING}]"),
            rule: "every file the register pins is a file this gate measures".to_owned(),
            violation: format!("{stale} is pinned and no longer here"),
            alternative: "delete the row: the debt it recorded has been paid or moved".to_owned(),
        });
    }
    violations.extend(spent_signatures(&excused, &still_here, arg_limit));
    Ok(violations)
}

/// The signatures the register still excuses and no longer needs to.
///
/// The parameter register used to be consulted and never audited, so a
/// name stayed on it after the clump had been given a name, and it
/// silently re-excused any function later born with that name in that
/// file. Both other registers in this repository - the file register
/// above and `boundary`'s - already report their own spent rows, and
/// this is the same rule in the same words.
fn spent_signatures(
    excused: &BTreeSet<String>,
    still_here: &BTreeMap<String, usize>,
    limit: usize,
) -> Vec<Violation> {
    let mut out = Vec::new();
    for named in excused {
        let violation = match still_here.get(named) {
            None => format!("{named} is excused and no longer here"),
            Some(&args) if args <= limit => {
                format!("{named} takes {args}, inside the {limit} the rule states")
            }
            Some(_) => continue,
        };
        out.push(Violation {
            gate: "length",
            location: format!("xtask/budgets.toml [{ARG_ROW}.{PREDATING}]"),
            rule: "an exception that is no longer needed is struck from the register".to_owned(),
            violation,
            alternative: "delete its row: the values that travelled together have a name now, \
                          and a spent exception left in place re-excuses whatever is written \
                          at that address next"
                .to_owned(),
        });
    }
    out
}

/// The file rule, applied to one file whatever language it is in: a file
/// the register does not name stays inside the budget, a file it names
/// may only get smaller, and a file back inside the budget loses its
/// row.
fn judge_file(rule: &FileRule, rel: &str, text: &str, out: &mut Vec<Violation>) {
    let lines = text.lines().count();
    match rule.predating.get(rel) {
        None => {
            if lines > rule.limit {
                out.push(too_long(rel, lines, rule.limit));
            }
        }
        Some(&pinned) => {
            if lines > pinned {
                out.push(grew(rel, lines, pinned));
            } else if lines <= rule.limit {
                out.push(no_longer_an_exception(rel, lines, rule.limit));
            }
        }
    }
}

/// True when a generator wrote this file and says so in it. Read by
/// `wording` as well: a sentence a generator wrote is judged where the
/// generator is.
pub(crate) fn generated(text: &str) -> bool {
    text.lines().take(10).any(|line| line.contains(GENERATED))
}

/// The client's own sources, the page shell and the type declarations
/// left out by extension.
fn client_sources(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let base = root.join(CLIENT_DIR);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    walk::files_with_ext(&base, &CLIENT_EXTENSIONS)
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
