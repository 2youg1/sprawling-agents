// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The last three assertions of the `spec` gate (xtask-SPEC.md section
//! 8-42), read off every `.lean` in the tree as text: the imports each
//! side may make, no proof obligation left undischarged, and every
//! repository path a specification cites on disk.
//!
//! **The checker and the specifications do not import each other**
//! (ARCHITECTURE.md section 11): the checker judges the binary from
//! outside, and a checker that imported a restatement of a rule would
//! hold a second authority for it. A specification follows the crate
//! graph, so a part cannot lean on a crate its own crate may not use.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::architecture;
use crate::depmap;
use crate::lean;
use crate::members::{self, Member};
use crate::release;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// Where the checker's Lean lives.
const CHECKER: &str = "tools/adversary/";

/// The library the checker is, and the one it may import besides the
/// toolchain's.
const CHECKER_LIBRARY: &str = "Sprawling";

/// The libraries the Lean toolchain ships, which anything may import.
const TOOLCHAIN: [&str; 3] = ["Init", "Std", "Lean"];

/// What a file answers to: the checker's rules, or those of the package
/// whose directory holds it.
#[derive(Clone, Copy)]
enum Owner<'a> {
    Checker,
    Package(&'a Member),
}

pub(super) fn check(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let found = members::members(root)?;
    let allowed = depmap::parse_block(&walk::read_text(&root.join(architecture::PATH))?)?;
    let top = top_directories(root)?;
    for file in walk::files_with_ext(root, &["lean"])? {
        let rel = walk::rel(root, &file);
        if walk::in_isolation_zone(&rel) {
            continue;
        }
        let text = walk::read_text(&file)?;
        let code = lean::code(&text);
        undischarged(&rel, &code, out);
        let owner = if rel.starts_with(CHECKER) {
            Some(Owner::Checker)
        } else {
            found
                .iter()
                .find(|member| member.holds(&rel))
                .map(Owner::Package)
        };
        let Some(owner) = owner else {
            continue;
        };
        let reach = Reach {
            found: &found,
            allowed: &allowed,
        };
        for (line, module) in lean::imports(&code) {
            if let Some(why) = reach.refusal(owner, &module) {
                out.push(finding(
                    format!("{rel}:{line}"),
                    "the checker and the specifications import along the crate graph and \
                     never each other (xtask-SPEC.md section 8-42)",
                    why,
                    "import the toolchain's libraries, this crate's parts, or the parts of a \
                     crate the depmap block lets this one depend on",
                ));
            }
        }
        if let Owner::Package(_) = owner {
            out.extend(cited_paths(root, &rel, &text, &top));
        }
    }
    Ok(())
}

/// A `sorry` or an `admit` left in the code, or an `axiom` declared.
///
/// A word keeps its dots, so `Door.admit` and `d.admit` are names rather
/// than the tactic; `admit` is the tactic only where nothing but the end
/// of the line, a `;` or a `)` follows it, since a binder or a field may
/// be called `admit` too. `sorry` is a term Lean reserves, so any
/// occurrence is one.
fn undischarged(rel: &str, code: &str, out: &mut Vec<Violation>) {
    for (index, line) in code.lines().enumerate() {
        let words: Vec<&str> = line
            .split(|c: char| !word(c))
            .filter(|found| !found.is_empty())
            .collect();
        let axiom = matches!(words.as_slice(), ["axiom", ..] | ["private", "axiom", ..]);
        let why = match (axiom, words.contains(&"sorry"), tactic_admit(line)) {
            (true, _, _) => "declares an axiom",
            (false, true, _) => "leaves a `sorry`",
            (false, false, true) => "leaves an `admit`",
            (false, false, false) => continue,
        };
        out.push(finding(
            format!("{rel}:{}", index.saturating_add(1)),
            "a specification proves what it states: no `sorry`, `admit` or `axiom` \
             (xtask-SPEC.md section 8-42)",
            why.to_owned(),
            "prove it as a theorem; an assumption the proof needs is a hypothesis of the \
             theorem, where a reader sees it",
        ));
    }
}

/// A character of a Lean name, dots included.
fn word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '\'' | '.')
}

/// Whether `line` uses the `admit` tactic.
fn tactic_admit(line: &str) -> bool {
    const ADMIT: &str = "admit";
    line.match_indices(ADMIT).any(|(at, _)| {
        let before = line.get(..at).and_then(|head| head.chars().next_back());
        let after = line
            .get(at.saturating_add(ADMIT.len())..)
            .unwrap_or_default()
            .trim_start();
        !before.is_some_and(word) && (after.is_empty() || after.starts_with([';', ')']))
    })
}

/// The depmap block and the packages it names, which together say what
/// a file may import.
struct Reach<'a> {
    found: &'a [Member],
    allowed: &'a BTreeMap<String, BTreeSet<String>>,
}

impl Reach<'_> {
    /// Why `owner` may not import `module`, or `None` when it may.
    fn refusal(&self, owner: Owner<'_>, module: &str) -> Option<String> {
        if TOOLCHAIN.iter().any(|library| within(module, library)) {
            return None;
        }
        let own = match owner {
            Owner::Checker => {
                return (!within(module, CHECKER_LIBRARY)).then(|| {
                    format!(
                        "the checker imports `{module}`, and it imports only `{CHECKER_LIBRARY}` \
                         and the toolchain's libraries"
                    )
                });
            }
            Owner::Package(own) => own,
        };
        let Some(target) = self
            .found
            .iter()
            .filter(|member| within(module, &lean::dotted(&member.dir)))
            .max_by_key(|member| member.dir.len())
        else {
            return Some(format!(
                "imports `{module}`, which is neither the toolchain's nor a crate's specification"
            ));
        };
        let permitted = target.dir == own.dir
            || self
                .allowed
                .get(own.name())
                .is_some_and(|deps| deps.contains(target.name()));
        (!permitted).then(|| {
            format!(
                "imports `{module}`, a part of {}, which the depmap block does not let {} depend on",
                target.name(),
                own.name()
            )
        })
    }
}

/// Whether `module` is `prefix` or a module below it.
fn within(module: &str, prefix: &str) -> bool {
    module
        .strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

/// A repository path the specification cites whose file is not there.
fn cited_paths(root: &Path, rel: &str, text: &str, top: &BTreeSet<String>) -> Vec<Violation> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for token in release::path_tokens(line) {
            let path = token.trim_end_matches(['.', '-']);
            let cited = release::CITED_EXTENSIONS
                .iter()
                .any(|ext| path.ends_with(ext));
            let anchored = path
                .split_once('/')
                .is_some_and(|(first, _)| top.contains(first));
            if !cited || !anchored || root.join(path).is_file() {
                continue;
            }
            out.push(finding(
                format!("{rel}:{}", index.saturating_add(1)),
                "a specification cites only files on disk (xtask-SPEC.md section 8-42)",
                format!("cites `{path}`, which is not there"),
                "follow the file to where it moved, or say what replaced it",
            ));
        }
    }
    out
}

/// The directories at the repository root: a cited path that starts at
/// one of them is a path into this tree.
fn top_directories(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let io = |source| XtaskError::Io {
        path: ".".to_owned(),
        source,
    };
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(root).map_err(io)? {
        let entry = entry.map_err(io)?;
        if entry.path().is_dir() {
            out.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(out)
}

fn finding(location: String, rule: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "spec",
        location,
        rule: rule.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}
