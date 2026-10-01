// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The class column: which class of verb each Command is when a remote
//! device sends it (crates/wire/Spec.lean section 19-2,
//! tools/xtask/Spec.lean §8-45).
//!
//! **Two sources, one decision.** The arms of `def Command.verbClass` in
//! the wire specification state each verb's class, and the relay decides
//! it with an exhaustive match, `command_class` in the binary's package.
//! Neither can be derived from the other: the table is what a reader is
//! told, the match is what a device meets. The gate reads both and says
//! where they part, so the table cannot drift from the relay without a
//! red.
//!
//! A verb with no class arm is named by its verb. Lean refuses to build a
//! `def` that leaves out a constructor, but the gate runs without Lean,
//! so the same finding is made here; the decision the table records for
//! every new Command is `LocalOnly`.

use std::collections::BTreeMap;
use std::path::Path;

use super::{WIRE_CRATE, violation};
use crate::lean;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// The package whose `command_class` the relay decides with.
const RELAY: &str = "sprawling";
/// The function whose arms are the relay's classes.
const CLASSIFIER: &str = "fn command_class";
/// The `def` the wire specification writes the class column as.
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
    /// One spelling, the Rust one, on both sides (tools/xtask/Spec.lean
    /// §8-43): the relay's `VerbClass::Act` and the table's `.Act`.
    fn parse(word: &str) -> Option<Self> {
        match word {
            "Read" => Some(Self::Read),
            "Act" => Some(Self::Act),
            "LocalOnly" => Some(Self::LocalOnly),
            _ => None,
        }
    }
}

/// What the wire specification states, verb by verb, and the Lean file
/// that states it.
pub(super) struct Stated {
    pub(super) path: String,
    /// Each verb's class, or `None` for an arm that answers something that
    /// is not a class.
    pub(super) classes: BTreeMap<String, Option<Class>>,
}

/// The arms of `def Command.verbClass`. With no such `def` every verb is
/// unclassified, and the findings are located at the entry.
pub(super) fn stated(root: &Path) -> Result<Stated, XtaskError> {
    let sources = super::reading::specification(root)?;
    Ok(sources
        .iter()
        .find_map(|source| {
            lean::arms(&lean::code(&source.text), CLASS_DEF).map(|table| Stated {
                path: source.path.clone(),
                classes: table
                    .arms
                    .into_iter()
                    .map(|arm| {
                        let class = arm.value.strip_prefix('.').and_then(Class::parse);
                        (arm.pattern, class)
                    })
                    .collect(),
            })
        })
        .unwrap_or_else(|| Stated {
            path: format!("{WIRE_CRATE}/{}", lean::ENTRY),
            classes: BTreeMap::new(),
        }))
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
    stated: &Stated,
    coded: &BTreeMap<String, Class>,
) -> Vec<Violation> {
    let mut out = Vec::new();
    for name in all {
        match (stated.classes.get(name).copied().flatten(), coded.get(name)) {
            (None, _) => out.push(violation(
                &stated.path,
                "every Command states the class a remote device's frame carries",
                format!(
                    "`{name}` has no class in `def Command.verbClass` (crates/wire/Spec.lean \
                     section 19-2)"
                ),
                "write `.LocalOnly` as its arm unless a person decided a device away from the \
                 machine may do it; `.Act` and `.Read` are decisions, not defaults"
                    .to_owned(),
            )),
            (Some(table), Some(relay)) if table != *relay => out.push(violation(
                &stated.path,
                "the class table and the relay decide one thing",
                format!("`{name}` is {table:?} in the table and {relay:?} in `command_class`"),
                "change whichever one is wrong: the table is what a reader is told, the match \
                 is what a remote device meets"
                    .to_owned(),
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

    /// An arm that answers something other than a class names its verb -
    /// the lower-case spelling the table once allowed included - and a
    /// class the relay decides otherwise is named too, both located at the
    /// file that holds the arms.
    #[test]
    fn a_verb_whose_class_arm_is_not_a_class_is_named_by_its_verb() {
        let root = crate::root::fixture::relocated("wiring-class");
        crate::root::fixture::write(
            &root,
            "crates/wire/Spec.lean",
            "def Command.verbClass : Command → VerbClass\n  | .Dispatch => .Act\n  \
             | .Reveal => .LocalOnly\n  | .Gift => .localOnly\n",
        );
        let read = stated(&root);
        std::fs::remove_dir_all(&root).unwrap();
        let all = ["Dispatch", "Reveal", "Gift"].map(str::to_owned);
        let coded = arms(
            "fn command_class(c: &C) -> VerbClass {\n    match c {\n        \
             wire::Command::Dispatch { .. } => VerbClass::Act,\n        \
             wire::Command::Reveal { .. }\n        | wire::Command::Gift(_) => VerbClass::Act,\n    }\n}\n",
        );
        let named: Vec<(String, String)> = judged(&all, &read.unwrap(), &coded)
            .into_iter()
            .map(|found| (found.location, found.violation))
            .collect();
        let at = "crates/wire/Spec.lean".to_owned();
        assert_eq!(
            named,
            [
                (
                    at.clone(),
                    "`Reveal` is LocalOnly in the table and Act in `command_class`".to_owned()
                ),
                (
                    at,
                    "`Gift` has no class in `def Command.verbClass` (crates/wire/Spec.lean \
                     section 19-2)"
                        .to_owned()
                ),
            ]
        );
    }
}
