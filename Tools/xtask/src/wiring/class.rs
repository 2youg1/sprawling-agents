// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use super::{WIRE_CRATE, violation};
use crate::lean;
use crate::report::{Violation, XtaskError};
use crate::walk;

const RELAY: &str = "sprawling";

const CLASSIFIER: &str = "fn command_class";

const CLASS_DEF: &str = "Command.verbClass";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    Read,
    Act,
    LocalOnly,
}

impl Class {

    fn parse(word: &str) -> Option<Self> {
        match word {
            "Read" => Some(Self::Read),
            "Act" => Some(Self::Act),
            "LocalOnly" => Some(Self::LocalOnly),
            _ => None,
        }
    }
}

pub(super) struct Stated {
    pub(super) path: String,

    pub(super) classes: BTreeMap<String, Option<Class>>,
}

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
                    "`{name}` has no class in `def Command.verbClass` (Sprawling/wire/Spec.lean \
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

    #[test]
    fn a_verb_whose_class_arm_is_not_a_class_is_named_by_its_verb() {
        let root = crate::root::fixture::relocated("wiring-class");
        crate::root::fixture::write(
            &root,
            "Sprawling/wire/Spec.lean",
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
        let at = "Sprawling/wire/Spec.lean".to_owned();
        assert_eq!(
            named,
            [
                (
                    at.clone(),
                    "`Reveal` is LocalOnly in the table and Act in `command_class`".to_owned()
                ),
                (
                    at,
                    "`Gift` has no class in `def Command.verbClass` (Sprawling/wire/Spec.lean \
                     section 19-2)"
                        .to_owned()
                ),
            ]
        );
    }
}
