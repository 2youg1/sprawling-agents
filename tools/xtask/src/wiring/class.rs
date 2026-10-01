// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The class column: which class of verb each Command is when a remote
//! device sends it (wire-SPEC.md section 19-2, xtask-SPEC.md section
//! 8-45).
//!
//! **Two sources, one decision.** The SPEC's table states each verb's
//! class, and the relay decides it with an exhaustive match,
//! `command_class` in the binary's package. Neither can be derived from
//! the other: the table is what a reader is told, the match is what a
//! device meets. The gate reads both and says where they part, so the
//! table cannot drift from the relay without a red.
//!
//! A row with no class cell is named by its verb. That is the shape a
//! row takes when a change adds a Command and writes its reach without
//! deciding what a remote device may do with it, and the decision the
//! table records for every new Command is `LocalOnly`.

use std::collections::BTreeMap;
use std::path::Path;

use super::{SPEC, WIRE_CRATE, violation};
use crate::lean;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// The package whose `command_class` the relay decides with.
const RELAY: &str = "sprawling";
/// The function whose arms are the relay's classes.
const CLASSIFIER: &str = "fn command_class";
/// The `def` a migrated wire specification writes the class column as.
const CLASS_DEF: &str = "Command.verbClass";

/// The three classes, spelled as `remote_access::door::VerbClass` spells
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    Read,
    Act,
    LocalOnly,
}

impl Class {
    fn parse(cell: &str) -> Option<Self> {
        match cell.trim().trim_matches('`') {
            "Read" | "read" => Some(Self::Read),
            "Act" | "act" => Some(Self::Act),
            "LocalOnly" | "localOnly" => Some(Self::LocalOnly),
            _ => None,
        }
    }
}

/// What the SPEC states, verb by verb: its class, or `None` for a row
/// whose class cell is missing or is not a class.
pub(super) fn stated(root: &Path) -> Result<BTreeMap<String, Option<Class>>, XtaskError> {
    match lean::specification(root, WIRE_CRATE)? {
        Some(sources) => Ok(sources
            .iter()
            .find_map(|source| lean::arms(&lean::code(&source.text), CLASS_DEF))
            .map(|table| {
                table
                    .arms
                    .into_iter()
                    .map(|arm| {
                        let class = arm.value.strip_prefix('.').and_then(Class::parse);
                        (arm.pattern, class)
                    })
                    .collect()
            })
            .unwrap_or_default()),
        None => Ok(markdown(&walk::read_text(&root.join(SPEC))?)),
    }
}

/// The reach rows of the Markdown table, each with its third cell read
/// as a class.
fn markdown(text: &str) -> BTreeMap<String, Option<Class>> {
    text.lines()
        .filter_map(super::reading::cells)
        .filter_map(|cells| {
            let name = cells
                .first()?
                .strip_prefix('`')
                .and_then(|cell| cell.strip_suffix('`'))?
                .to_owned();
            // A reach row and nothing else: the document holds other
            // tables with a backticked first column.
            super::Reach::parse(cells.get(1)?)?;
            Some((name, cells.get(2).and_then(|cell| Class::parse(cell))))
        })
        .collect()
}

/// What the relay's match decides, verb by verb.
pub(super) fn coded(root: &Path) -> Result<BTreeMap<String, Class>, XtaskError> {
    let src = format!(
        "{}/src",
        crate::members::find(&crate::members::members(root)?, RELAY)?.dir
    );
    for file in walk::files_with_ext(&root.join(&src), &["rs"])? {
        let text = walk::read_text(&file)?;
        if let Some(start) = text.find(CLASSIFIER) {
            return Ok(arms(text.get(start..).unwrap_or_default()));
        }
    }
    Err(XtaskError::Doc {
        file: src,
        msg: format!("no `{CLASSIFIER}` under here to read the relay's classes from"),
    })
}

/// Reads `Command::<Verb>` patterns up to each `=> VerbClass::<Class>`,
/// to the end of the function.
fn arms(body: &str) -> BTreeMap<String, Class> {
    let body = body
        .find("\n}")
        .and_then(|end| body.get(..end))
        .unwrap_or(body);
    let mut out = BTreeMap::new();
    let mut waiting: Vec<String> = Vec::new();
    for (index, piece) in body.split("Command::").enumerate() {
        if index == 0 {
            continue;
        }
        let verb: String = piece
            .chars()
            .take_while(|each| each.is_alphanumeric() || *each == '_')
            .collect();
        waiting.push(verb);
        let decided = piece.split_once("=> VerbClass::").and_then(|(_, rest)| {
            Class::parse(rest.split(|each: char| !each.is_alphanumeric()).next()?)
        });
        if let Some(class) = decided {
            out.extend(waiting.drain(..).map(|verb| (verb, class)));
        }
    }
    out
}

/// A violation for every verb whose class is missing from the table or
/// disagrees with the relay.
pub(super) fn judged(
    all: &[String],
    stated: &BTreeMap<String, Option<Class>>,
    coded: &BTreeMap<String, Class>,
) -> Vec<Violation> {
    let mut out = Vec::new();
    for name in all {
        match (stated.get(name).copied().flatten(), coded.get(name)) {
            (None, _) => out.push(violation(
                "every Command states the class a remote device's frame carries",
                format!("the row for `{name}` in wire-SPEC.md section 19-2 has no class"),
                "write `LocalOnly` in its class cell unless a person decided a device away \
                 from the machine may do it; `Act` and `Read` are decisions, not defaults",
            )),
            (Some(table), Some(relay)) if table != *relay => out.push(violation(
                "the class table and the relay decide one thing",
                format!("`{name}` is {table:?} in the table and {relay:?} in `command_class`"),
                "change whichever one is wrong: the table is what a reader is told, the match \
                 is what a remote device meets",
            )),
            (Some(_), Some(_) | None) => {}
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    /// A row a change added with its reach and no class names its verb,
    /// and a class the relay decides otherwise is named too.
    #[test]
    fn a_row_with_no_class_cell_is_named_by_its_verb() {
        let root = crate::root::fixture::relocated("wiring-class");
        crate::root::fixture::write(
            &root,
            SPEC,
            "| Command | reach | class | 说明 |\n|---|---|---|---|\n\
             | `Dispatch` | client | Act | work |\n\
             | `Reveal` | client | LocalOnly | a folder |\n\
             | `Gift` | client | a verb added without its class |\n",
        );
        let read = stated(&root);
        std::fs::remove_dir_all(&root).unwrap();
        let all = ["Dispatch", "Reveal", "Gift"].map(str::to_owned);
        let coded = arms(
            "fn command_class(c: &C) -> VerbClass {\n    match c {\n        \
             wire::Command::Dispatch { .. } => VerbClass::Act,\n        \
             wire::Command::Reveal { .. }\n        | wire::Command::Gift(_) => VerbClass::Act,\n    }\n}\n",
        );
        let named: Vec<String> = judged(&all, &read.unwrap(), &coded)
            .into_iter()
            .map(|found| found.violation)
            .collect();
        assert_eq!(
            named,
            [
                "`Reveal` is LocalOnly in the table and Act in `command_class`",
                "the row for `Gift` in wire-SPEC.md section 19-2 has no class",
            ]
        );
    }
}
