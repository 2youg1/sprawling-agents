// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The first two assertions of the `spec` gate (tools/xtask/Spec.lean
//! §8-42): a package holds exactly one effective specification, and prose
//! names no SPEC the tree does not have.
//!
//! A migration that stops halfway leaves two specifications a builder
//! has to choose between (`skills/sdd`, migration step 5); a migration
//! that deleted the Markdown SPEC and left its citations behind sends a
//! reader to a file that is gone. The second has already happened
//! without any migration: a gate's rustdoc cited a `web` SPEC long after
//! that crate stopped existing.

use std::collections::BTreeSet;
use std::path::Path;

use crate::lean;
use crate::members::{self, Member};
use crate::release;
use crate::report::{Violation, XtaskError};

/// How every Markdown SPEC's file name ends.
const MARKDOWN: &str = "-SPEC.md";

/// How a SPEC is cited in prose, with or without its extension.
const CITED: &str = "-SPEC";

/// The one document whose SPEC names are history: each of its sections
/// describes the tree of that release.
const HISTORY: &str = "CHANGELOG.md";

/// The extensions whose prose lines `release::is_prose` can tell apart.
const PROSE: [&str; 4] = [".md", ".html", ".rs", ".lean"];

pub(super) fn one_per_package(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    for member in members::members(root)? {
        match held_in(root, &member.dir)?.as_slice() {
            [_] => {}
            [] => out.push(finding(
                member.dir.clone(),
                "every package has a specification (tools/xtask/Spec.lean §8-42)",
                format!(
                    "{} holds neither a `*{MARKDOWN}` nor a {}",
                    member.dir,
                    lean::ENTRY
                ),
                format!(
                    "run `cargo xtask spec {}` and write the package's specification",
                    member.name()
                ),
            )),
            held @ [..] => out.push(two_held(&member.dir, held)),
        }
    }
    // The checker is no package, and its specification is held to the same
    // count; having none is not judged here, because no cargo package
    // promises that the directory exists.
    if let held @ [_, _, ..] = held_in(root, super::CHECKER)?.as_slice() {
        out.push(two_held(super::CHECKER, held));
    }
    Ok(())
}

/// The specifications sitting in `dir`: its Markdown SPECs, then its
/// `Spec.lean` when it has migrated.
fn held_in(root: &Path, dir: &str) -> Result<Vec<String>, XtaskError> {
    let mut held = markdown_in(root, dir)?;
    if lean::migrated(root, dir) {
        held.push(format!("{dir}/{}", lean::ENTRY));
    }
    Ok(held)
}

/// A directory holding more than one effective specification.
fn two_held(dir: &str, held: &[String]) -> Violation {
    finding(
        dir.to_owned(),
        "a package has one effective specification (tools/xtask/Spec.lean §8-42)",
        format!("{dir} holds {}", held.join(" and ")),
        "finish the migration in one change-set: carry every requirement into Spec.lean, \
         then delete the Markdown SPEC"
            .to_owned(),
    )
}

/// The Markdown SPECs sitting directly in `dir`, repo-relative, sorted.
fn markdown_in(root: &Path, dir: &str) -> Result<Vec<String>, XtaskError> {
    let io = |source| XtaskError::Io {
        path: dir.to_owned(),
        source,
    };
    let mut held = Vec::new();
    if !root.join(dir).is_dir() {
        return Ok(held);
    }
    for entry in std::fs::read_dir(root.join(dir)).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(MARKDOWN) && entry.path().is_file() {
            held.push(format!("{dir}/{name}"));
        }
    }
    held.sort();
    Ok(held)
}

pub(super) fn no_dangling_names(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let published = release::published(root)?;
    let present: BTreeSet<&str> = published
        .iter()
        .filter_map(|rel| rel.rsplit('/').next())
        .filter(|name| name.ends_with(MARKDOWN))
        .collect();
    let found = members::members(root)?;
    for rel in &published {
        if rel == HISTORY || !PROSE.iter().any(|ext| rel.ends_with(ext)) {
            continue;
        }
        // Bytes that are not UTF-8 are fixtures, as `release` reads them.
        let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if !release::is_prose(rel, line) {
                continue;
            }
            for stem in cited(line) {
                if present.contains(format!("{stem}{MARKDOWN}").as_str()) {
                    continue;
                }
                out.push(finding(
                    format!("{rel}:{}", index.saturating_add(1)),
                    "prose names only a SPEC this tree has (tools/xtask/Spec.lean §8-42)",
                    format!("names `{stem}{CITED}`, and no `{stem}{MARKDOWN}` is in the tree"),
                    successor(root, &found, stem),
                ));
            }
        }
    }
    Ok(())
}

/// Every `<name>-SPEC` a line names, with or without `.md`. A name made
/// of anything but letters, digits and underscores is a pattern such as
/// `<lib>-SPEC.md`, not a citation.
fn cited(line: &str) -> Vec<&str> {
    let part = |c: char| c.is_ascii_alphanumeric() || c == '_';
    line.match_indices(CITED)
        .filter_map(|(at, _)| {
            let after = line.get(at.saturating_add(CITED.len())..)?;
            if after.starts_with(part) {
                return None;
            }
            let head = line.get(..at)?;
            let start = head
                .char_indices()
                .rev()
                .take_while(|(_, c)| part(*c))
                .last()
                .map(|(index, _)| index)?;
            head.get(start..)
        })
        .collect()
}

/// Where a reader should look instead: the Lean specification of the
/// package of that name, when it has migrated.
fn successor(root: &Path, found: &[Member], stem: &str) -> String {
    found
        .iter()
        .find(|member| {
            member.dir.rsplit('/').next() == Some(stem) && lean::migrated(root, &member.dir)
        })
        .map_or_else(
            || "correct the name, or delete the sentence: a reader cannot open it".to_owned(),
            |member| {
                format!(
                    "that crate's specification is {}/{}; cite it, or the decision as `{} D<n>`",
                    member.dir,
                    lean::ENTRY,
                    member.name()
                )
            },
        )
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
