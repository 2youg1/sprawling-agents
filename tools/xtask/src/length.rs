// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Length gate: a function body nobody reads to the end, and a
//! parameter list whose values travel together without a name, both turn
//! the build red.
//!
//! **The unit is the function, not the file.** A long function hides a
//! flow of control; across the production functions of this tree the
//! median is 9 lines and p99 is under 150, so the 200-line budget sits
//! well above where the repository writes and catches a whole flow of
//! control written into one body. A file is not measured: a cap on file
//! length forced a split of a file that was correct, which is a cost
//! every change paid and no defect any split removed
//! (tools/xtask/Spec.lean §10-7).
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
//! with no branches), because a table is looked things up in rather
//! than read through.
//!
//! **Zig functions are measured too.** The Zig leaf's functions are held
//! to the same function budget as Rust, which is the fifth condition
//! ARCHITECTURE.md section 2 sets for admitting a leaf. Zig needs no
//! parser for that: its lexical grammar has no block comment and no
//! string that crosses a line, so `zig` reads the brace structure
//! exactly once four kinds of token are skipped. The parameter rule
//! stays Rust's, because a Zig file here is a C ABI leaf whose
//! parameters are `(ptr, len)` pairs and Win32's own signatures
//! (xtask D15).
//!
//! **The parameter register cleans itself.** `[argument_count.predating]`
//! names the signatures that predate the rule; a name not on it is
//! refused outright, and a name that has come back inside the budget or
//! left the tree is reported so its row is struck.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::budget::REGISTER;
use crate::modmap;
use crate::report::{Violation, XtaskError};
use crate::walk;

mod measurement;
mod register;
mod zig;

use measurement::{Body, Found, measure};
use register::{ARG_ROW, PREDATING, ROW, excused, key, limit};

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let body_limit = limit(root, ROW)?;
    let arg_limit = limit(root, ARG_ROW)?;
    let excused = excused(root)?;
    let shapes = modmap::shapes(root)?;
    let mut violations = Vec::new();
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
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        for found in measure(&parsed.items) {
            if found.body.lines > body_limit {
                violations.push(over(&rel, &found.body, body_limit));
            }
            let named = key(&rel, &found.body.name);
            if excused.contains(&named) {
                let widest = still_here.entry(named).or_insert(found.args);
                *widest = (*widest).max(found.args);
            } else if found.args > arg_limit {
                violations.push(too_many_arguments(&rel, &found, arg_limit));
            }
        }
    }
    for file in zig_sources(root)? {
        let rel = walk::rel(root, &file);
        if shapes.get(&rel).is_some_and(|shape| shape == "data") {
            continue;
        }
        let text = walk::read_text(&file)?;
        violations.extend(
            zig::measure(&text)
                .iter()
                .filter(|body| body.lines > body_limit)
                .map(|body| over(&rel, body, body_limit)),
        );
    }
    violations.extend(spent_signatures(&excused, &still_here, arg_limit));
    Ok(violations)
}

/// The signatures the register still excuses and no longer needs to.
///
/// A register that is consulted and never audited keeps a name after
/// the clump has been given a name, and silently re-excuses any
/// function later born with that name in that file. `boundary`'s
/// register reports its own spent rows in the same words.
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
            location: format!("{REGISTER} [{ARG_ROW}.{PREDATING}]"),
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

fn too_many_arguments(rel: &str, found: &Found, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("{rel}:{}", found.body.line),
        rule: format!(
            "a function takes at most {limit} parameters \
             ({REGISTER}, {ARG_ROW})"
        ),
        violation: format!("{} takes {}", found.body.name, found.args),
        alternative: "the ones that always travel together are one value: give them a struct \
                      with a name, as `Reporter` did for the four that describe who is \
                      reporting a change to a plan"
            .to_owned(),
    }
}

fn over(rel: &str, body: &Body, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("{rel}:{}", body.line),
        rule: format!(
            "a production function stays inside {limit} lines \
             ({REGISTER}, function_length)"
        ),
        violation: format!("{} is {} lines", body.name, body.lines),
        alternative: "give a phase of it its own name, and hand the values it produces back \
                      as one value"
            .to_owned(),
    }
}

/// Where first-party Rust lives: the `src/` of every package cargo
/// places. `tests/` and `benches/` are absent on purpose: test code may
/// relax what production code carries (AGENTS.md), and a long test is a
/// different question from a long function.
fn sources(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let mut out = Vec::new();
    for member in crate::members::members(root)? {
        let base = root.join(&member.dir).join("src");
        if base.is_dir() {
            out.extend(walk::files_with_ext(&base, &["rs"])?);
        }
    }
    Ok(out)
}

/// Where first-party Zig lives: anywhere in a package's directory, since
/// a Zig leaf keeps its sources beside its manifest rather than under
/// `src/`. A package nested in another's directory is walked twice, and
/// its files are measured once.
fn zig_sources(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let members = crate::members::members(root)?;
    walk::files_under(
        root,
        members.iter().map(|member| member.dir.as_str()),
        &["zig"],
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
