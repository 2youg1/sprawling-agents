// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

const WIRE_CRATE: &str = "Sprawling/wire";

const TABLE: &str = "`def Command.reach` (Sprawling/wire/Spec.lean section 19-2)";
const WIRE_DIR: &str = "Sprawling/wire/src";

const WORKER: &str = "sprawling-accounting";

use crate::walk::CLIENT_SRC as CLIENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    Client,

    Push,

    Handshake,

    Sealed,
}

impl Reach {
    fn parse(cell: &str) -> Option<Self> {
        match cell {
            "client" => Some(Self::Client),
            "push" => Some(Self::Push),
            "handshake" => Some(Self::Handshake),
            "sealed" => Some(Self::Sealed),
            _ => None,
        }
    }
}

mod class;
mod reading;
mod settings;

use reading::{declared, variants};

fn violation(location: &str, rule: &str, subject: String, alternative: String) -> Violation {
    Violation {
        gate: "wiring",
        location: location.to_owned(),
        rule: rule.to_owned(),
        violation: subject,
        alternative,
    }
}

fn performed(root: &Path, all: &[String]) -> Result<BTreeSet<String>, XtaskError> {
    let worker_src = format!(
        "{}/src",
        crate::members::find(&crate::members::members(root)?, WORKER)?.dir
    );
    let mut found = None;
    for file in walk::files_with_ext(&root.join(&worker_src), &["rs"])? {
        let text = walk::read_text(&file)?;
        if let Some(start) = text.find("fn run_command") {
            found = Some(text.get(start..).unwrap_or_default().to_owned());
            break;
        }
    }
    let Some(body) = found else {
        return Err(XtaskError::Doc {
            file: worker_src,
            msg: "no `run_command` under here to read the arms of".to_owned(),
        });
    };
    let body = body.as_str();
    let mut out = BTreeSet::new();
    for name in all {
        let arm = format!("wire::Command::{name}");
        let Some(at) = arm_start(body, &arm) else {
            continue;
        };

        let rest = body.get(at.saturating_add(arm.len())..).unwrap_or_default();
        let end = rest
            .find("\n            wire::Command::")
            .unwrap_or(rest.len());
        if !rest.get(..end).unwrap_or_default().contains("not_built") {
            out.insert(name.clone());
        }
    }
    Ok(out)
}

fn wire_tag(variant: &str) -> String {
    let mut tag = String::new();
    for (index, character) in variant.char_indices() {
        if character.is_ascii_uppercase() && index != 0 {
            tag.push('_');
        }
        tag.extend(character.to_lowercase());
    }
    tag
}

fn arm_start(body: &str, arm: &str) -> Option<usize> {
    let mut from: usize = 0;
    loop {
        let found = body.get(from..)?.find(arm)?;
        let at = from.checked_add(found)?;
        let after = at.checked_add(arm.len())?;
        let follows_on = body
            .get(after..)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|next| next.is_alphanumeric() || next == '_');
        if !follows_on {
            return Some(at);
        }
        from = after;
    }
}

fn emitted(root: &Path, all: &[String]) -> Result<BTreeSet<String>, XtaskError> {
    let mut seen = BTreeSet::new();
    let base = root.join(CLIENT);
    if !base.exists() {
        return Ok(seen);
    }
    let tags: Vec<(String, String)> = all
        .iter()
        .map(|name| (name.clone(), format!("{}: {{", wire_tag(name))))
        .collect();
    for file in walk::files_with_ext(&base, &["ts", "tsx"])? {
        if walk::rel(root, &file).contains(".test.") {
            continue;
        }
        let text = walk::read_text(&file)?;
        for (name, spelling) in &tags {
            if text.contains(spelling) {
                seen.insert(name.clone());
            }
        }
    }
    Ok(seen)
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut settings = settings::check(root)?;
    let all = variants(root, "Command")?;
    let table = declared(root)?;
    let at = table.path.as_str();
    let performed = performed(root, &all)?;
    let emitted = emitted(root, &all)?;
    let mut violations = class::judged(&all, &class::stated(root)?, &class::coded(root)?);

    for name in &all {
        let Some(reach) = table.rows.get(name) else {
            violations.push(violation(
                at,
                "every Command states which side reaches it",
                format!("`{name}` is on the wire and absent from the reach table"),
                format!(
                    "add an arm to {TABLE}: a verb nobody classified is a verb nobody decided \
                     to expose"
                ),
            ));
            continue;
        };
        let drawn = emitted.contains(name);
        let built = performed.contains(name);
        match reach {
            Reach::Client if drawn && !built => violations.push(violation(
                at,
                "a control the client draws is a verb the city can carry out",
                format!("`{name}` is drawn and answered with `not_built`"),
                "take the control out until the executor exists: a refusal is what the city \
                 owes a peer that asks anyway, not a substitute for the button being gone"
                    .to_owned(),
            )),
            Reach::Client if !drawn && built => violations.push(violation(
                at,
                "a verb the city can carry out is reachable from the client",
                format!("`{name}` is built and no control reaches it"),
                format!(
                    "draw the control, or change its arm in {TABLE} and say why a person may \
                     not ask for it. A capability nobody can reach draws no complaint, because \
                     nobody fails to press a button that is not there"
                ),
            )),
            Reach::Sealed if drawn => violations.push(violation(
                at,
                "a sealed verb has no byte form, so no client may spell it",
                format!("`{name}` is sealed and the client names it"),
                "a credential reaches the vault in process; entering one remotely is meant to \
                 be unspellable"
                    .to_owned(),
            )),
            Reach::Push | Reach::Handshake if drawn => violations.push(violation(
                at,
                "a verb that arrives from outside is not a control",
                format!("`{name}` arrives by push or handshake and the client draws it"),
                format!("remove the control, or change its arm in {TABLE}"),
            )),
            _ => {}
        }
    }

    let known: BTreeSet<&String> = all.iter().collect();
    for named in table.rows.keys().filter(|name| !known.contains(name)) {
        violations.push(violation(
            at,
            "the reach table names verbs that are on the wire",
            format!("`{named}` is classified and is not a Command"),
            "delete the arm and its constructor: they survived the verb they described".to_owned(),
        ));
    }
    settings.extend(violations);
    Ok(settings)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        crate::root::this_checkout().to_path_buf()
    }

    #[test]
    fn the_wire_the_worker_and_the_client_are_all_readable() {
        let root = root();
        let all = variants(&root, "Command").unwrap();
        assert!(all.len() > 20, "the wire declares {} verbs", all.len());
        assert!(all.iter().any(|name| name == "Dispatch"));
        let performed = performed(&root, &all).unwrap();
        assert!(performed.contains("Dispatch"), "dispatch has an executor");
        assert!(
            !performed.contains("BatchByBuilding"),
            "batch-by-building is answered with not_built"
        );
        let emitted = emitted(&root, &all).unwrap();
        assert!(emitted.contains("Dispatch"), "the client draws dispatch");
    }

    #[test]
    fn the_reach_table_and_the_enum_are_the_same_list() {
        let root = root();
        let all: BTreeSet<String> = variants(&root, "Command").unwrap().into_iter().collect();
        let table: BTreeSet<String> = declared(&root).unwrap().rows.into_keys().collect();
        assert_eq!(all, table, "the enum and `def Command.reach` disagree");
    }

    #[test]
    fn a_reach_cell_reads_as_itself_and_nothing_else() {
        assert!(Reach::parse("client").is_some());
        assert!(Reach::parse("push").is_some());
        assert!(Reach::parse("handshake").is_some());
        assert!(Reach::parse("sealed").is_some());

        assert!(Reach::parse("value").is_none());
        assert!(Reach::parse("").is_none());
    }

    #[test]
    fn a_migrated_wire_specification_answers_the_reach_of_each_verb() {
        let root = crate::root::fixture::relocated("wiring-lean");
        crate::root::fixture::write(
            &root,
            "Sprawling/wire/Spec.lean",
            "/-! the reach -/\ndef Command.reach : Command → Reach\n  -- a person's verb\n  \
             | .Pursue => .client\n  | .Hello => .handshake\n\ndef other := 1\n",
        );
        let read = declared(&root);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            read.map(|table| (table.path, table.rows))
                .map_err(|err| err.to_string()),
            Ok((
                "Sprawling/wire/Spec.lean".to_owned(),
                std::collections::BTreeMap::from([
                    ("Hello".to_owned(), Reach::Handshake),
                    ("Pursue".to_owned(), Reach::Client),
                ])
            ))
        );
    }
}
