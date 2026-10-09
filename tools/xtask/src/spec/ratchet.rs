// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Assertions 6 and 7 of the `spec` gate (tools/xtask/Spec.lean §8-42):
//! two counts that may only fall, each pinned in `tools/xtask/budgets.toml`.
//! A count above its pin lists the offenders; a count below it asks for
//! the pin to be lowered in the same change, so the room it freed cannot
//! be spent again later.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::rustpath::Modules;
use super::theorems::{self, Class};
use crate::budget;
use crate::lean;
use crate::modmap;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// The two counts, each with its register row.
#[derive(Clone, Copy)]
enum Ratchet {
    /// Assertion 6: stateful parts that prove nothing.
    PartsWithoutTheorem,
    /// Assertion 7: backticked Rust paths that name nothing.
    UnresolvedPaths,
}

impl Ratchet {
    fn row(self) -> &'static str {
        match self {
            Self::PartsWithoutTheorem => "spec_parts_without_theorem",
            Self::UnresolvedPaths => "rust_paths_unresolved",
        }
    }

    fn fix(self) -> &'static str {
        match self {
            Self::PartsWithoutTheorem => {
                "state the part's transitions and prove a property over every trace, or leave \
                the count where it was"
            }
            Self::UnresolvedPaths => {
                "point the path at the item that exists now, or say what replaced it"
            }
        }
    }
}
/// The key both rows state their pin under.
const PINNED: &str = "pinned";
/// The module map shapes whose specification parts are counted (D23).
const STATEFUL: [&str; 2] = ["state machine", "typestate"];

/// The text files of the tree, read once, by repo-relative path.
pub(crate) struct Tree {
    pub(crate) lean: BTreeMap<String, String>,
    pub(crate) rust: BTreeMap<String, String>,
    pub(crate) markdown: BTreeMap<String, String>,
}

pub(super) fn check(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let tree = read_tree(root)?;
    let anchors = modmap::anchors(root)?;
    let shapes = modmap::shapes(root)?;
    let register = budget::register(root)?;
    let stateful: BTreeSet<String> = anchors
        .iter()
        .filter(|anchor| {
            shapes
                .get(&anchor.file)
                .is_some_and(|shape| STATEFUL.contains(&shape.as_str()))
        })
        .map(|anchor| anchor.spec.clone())
        .collect();
    let untheorem: Vec<String> = classes(&tree)
        .into_iter()
        .filter(|(part, class)| *class == Class::TextOnly && anchored(&stateful, part))
        .map(|(part, _)| part)
        .collect();
    compare(&register, Ratchet::PartsWithoutTheorem, &untheorem, out);
    let mut files = BTreeMap::new();
    for anchor in &anchors {
        // A row may note its role after the path, as `kernel::ledger (port)`.
        let module = anchor.module.split_whitespace().next().unwrap_or_default();
        files.insert(module.to_owned(), read_or_empty(root, &anchor.file)?);
        if let (Some(top), Some((dir, _))) =
            (module.split("::").next(), anchor.file.split_once("/src/"))
            && !files.contains_key(top)
        {
            for root_file in ["lib.rs", "main.rs"] {
                let path = format!("{dir}/src/{root_file}");
                if root.join(&path).is_file() {
                    files.insert(top.to_owned(), read_or_empty(root, &path)?);
                    break;
                }
            }
        }
    }
    let unresolved = unresolved(&Modules::new(files), &tree);
    compare(&register, Ratchet::UnresolvedPaths, &unresolved, out);
    Ok(())
}

/// Every specification part and its class (D22), by repo-relative path.
pub(crate) fn classes(tree: &Tree) -> BTreeMap<String, Class> {
    let checks: Vec<&String> = tree
        .rust
        .values()
        .filter(|text| theorems::holds_check(text))
        .collect();
    tree.lean
        .iter()
        .filter(|(rel, _)| is_part(rel))
        .map(|(rel, text)| {
            let found = theorems::theorems(&lean::code(text));
            let named: Vec<&&String> = checks.iter().filter(|t| t.contains(rel.as_str())).collect();
            let class = theorems::classify(&found, |name| {
                named.iter().any(|text| theorems::has_word(text, name))
            });
            (rel.clone(), class)
        })
        .collect()
}

/// Every backticked Rust path that resolves to nothing, as `file:line: path`.
pub(crate) fn unresolved(modules: &Modules, tree: &Tree) -> Vec<String> {
    let lean = tree
        .lean
        .iter()
        .flat_map(|(rel, text)| numbered(rel, text, |_| true));
    let markdown = tree
        .markdown
        .iter()
        .filter(|(rel, _)| !rel.ends_with("CHANGELOG.md"))
        .flat_map(|(rel, text)| numbered(rel, text, |_| true));
    let rustdoc = tree.rust.iter().flat_map(|(rel, text)| {
        numbered(rel, text, |line| {
            let line = line.trim_start();
            line.starts_with("///") || line.starts_with("//!")
        })
    });
    lean.chain(markdown)
        .chain(rustdoc)
        .flat_map(|(at, line)| {
            modules
                .paths_in(line)
                .into_iter()
                .filter(|path| !modules.resolves(path))
                .map(move |path| format!("{at}: `{path}`"))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The lines of `text` that `keep` accepts, each with `rel:line`.
fn numbered<'t>(
    rel: &'t str,
    text: &'t str,
    keep: impl Fn(&str) -> bool + 't,
) -> impl Iterator<Item = (String, &'t str)> + 't {
    text.lines()
        .enumerate()
        .filter(move |(_, line)| keep(line))
        .map(move |(index, line)| (format!("{rel}:{}", index.saturating_add(1)), line))
}

/// A `.lean` under a `spec/` directory, the checker's own code aside.
fn is_part(rel: &str) -> bool {
    let checker = format!("{}/", super::CHECKER);
    let checker_parts = format!("{checker}{}/", lean::PARTS);
    let mut dirs = rel.split('/').rev().skip(1);
    dirs.any(|dir| dir == lean::PARTS)
        && (!rel.starts_with(&checker) || rel.starts_with(&checker_parts))
}

/// Whether `part` is the file a stateful anchor names, or below the
/// directory of the same name.
fn anchored(stateful: &BTreeSet<String>, part: &str) -> bool {
    stateful.iter().any(|module| {
        let file = lean::module_file(module);
        let dir = file.trim_end_matches(".lean");
        part == file
            || part
                .strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// The pin of `row` against `found`: equal is green, anything else is a
/// violation that says which way to move.
fn compare(register: &toml::Value, ratchet: Ratchet, found: &[String], out: &mut Vec<Violation>) {
    let row = ratchet.row();
    let count = found.len();
    let pinned = register
        .get(row)
        .and_then(|table| table.get(PINNED))
        .and_then(toml::Value::as_integer)
        .and_then(|pin| usize::try_from(pin).ok());
    let location = format!("{}: [{row}]", budget::REGISTER);
    let rule = "a count the spec gate ratchets may only fall (tools/xtask/Spec.lean §8-42)";
    match pinned {
        None => out.push(finding(
            location,
            rule,
            format!("the register has no `[{row}] {PINNED}`, and the count today is {count}"),
            format!("add `[{row}]` with `{PINNED} = {count}`"),
        )),
        Some(pin) if count > pin => out.extend(found.iter().map(|offender| {
            finding(
                offender.clone(),
                rule,
                format!("[{row}] counts {count} against a pin of {pin}; this is one of them"),
                ratchet.fix().to_owned(),
            )
        })),
        Some(pin) if count < pin => out.push(finding(
            location,
            rule,
            format!("[{row}] counts {count}, below its pin of {pin}"),
            format!("lower `{PINNED}` to {count} in this change"),
        )),
        Some(_) => {}
    }
}

fn read_tree(root: &Path) -> Result<Tree, XtaskError> {
    let mut tree = Tree {
        lean: BTreeMap::new(),
        rust: BTreeMap::new(),
        markdown: BTreeMap::new(),
    };
    for file in walk::files(root)? {
        let rel = walk::rel(root, &file);
        let bucket = match file.extension().and_then(|ext| ext.to_str()) {
            Some("lean") => &mut tree.lean,
            Some("rs") => &mut tree.rust,
            Some("md") => &mut tree.markdown,
            Some(_) | None => continue,
        };
        if !walk::in_isolation_zone(&rel) {
            bucket.insert(rel, walk::read_text(&file)?);
        }
    }
    Ok(tree)
}

fn read_or_empty(root: &Path, rel: &str) -> Result<String, XtaskError> {
    let path = root.join(rel);
    if path.is_file() {
        walk::read_text(&path)
    } else {
        Ok(String::new())
    }
}

fn finding(location: String, rule: &str, violation: String, alternative: String) -> Violation {
    Violation {
        gate: "spec",
        location,
        rule: rule.to_owned(),
        violation,
        alternative,
    }
}
