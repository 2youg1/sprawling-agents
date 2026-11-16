// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::architecture;
use crate::depmap;
use crate::lean;
use crate::members::{self, Member};
use crate::release;
use crate::report::{Violation, XtaskError};
use crate::walk;

const CHECKER_LIBRARY: &str = "Sprawling";

const TOOLCHAIN: [&str; 3] = ["Init", "Std", "Lean"];

#[derive(Clone, Copy)]
enum Owner<'a> {
    Checker,
    CheckerSpecification,
    Package(&'a Member),
}

fn owner_of<'a>(found: &'a [Member], rel: &str) -> Option<Owner<'a>> {
    let Some(inside) = super::under_checker(rel) else {
        return members::owner(found, rel).map(Owner::Package);
    };
    let specified = inside == lean::ENTRY
        || inside
            .strip_prefix(lean::PARTS)
            .is_some_and(|rest| rest.starts_with('/'));
    Some(if specified {
        Owner::CheckerSpecification
    } else {
        Owner::Checker
    })
}

pub(super) fn check(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let found = members::members(root)?;
    let allowed = depmap::parse_block(&walk::read_text(&root.join(architecture::PATH))?)?;
    let top = top_directories(root)?;
    for file in walk::files_with_ext(root, &["lean"])? {
        let walked = walk::rel(root, &file);
        if walk::in_isolation_zone(&walked) {
            continue;
        }
        let rel = super::checker_rel(&walked);
        let text = walk::read_text(&file)?;
        let code = lean::code(&text);
        undischarged(&rel, &code, out);
        let Some(owner) = owner_of(&found, &rel) else {
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
                     never each other (Tools/xtask/Spec.lean §8-42)",
                    why,
                    "import the toolchain's libraries, this crate's parts, or the parts of a \
                     crate the depmap block lets this one depend on",
                ));
            }
        }
        match owner {
            Owner::Package(_) | Owner::CheckerSpecification => {
                out.extend(cited_paths(root, &rel, &text, &top));
            }
            Owner::Checker => {}
        }
    }
    Ok(())
}

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
             (Tools/xtask/Spec.lean §8-42)",
            why.to_owned(),
            "prove it as a theorem; an assumption the proof needs is a hypothesis of the \
             theorem, where a reader sees it",
        ));
    }
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '\'' | '.')
}

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

struct Reach<'a> {
    found: &'a [Member],
    allowed: &'a BTreeMap<String, BTreeSet<String>>,
}

impl Reach<'_> {
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
            Owner::CheckerSpecification => {
                let own = lean::dotted(super::CHECKER);
                let entry = format!("{own}.Spec");
                let parts = format!("{own}.{}", lean::PARTS);
                return (!(within(module, &entry) || within(module, &parts))).then(|| {
                    format!(
                        "the checker's specification imports `{module}`, and it imports only \
                         its own parts and the toolchain's libraries"
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

fn within(module: &str, prefix: &str) -> bool {
    module
        .strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

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
                "a specification cites only files on disk (Tools/xtask/Spec.lean §8-42)",
                format!("cites `{path}`, which is not there"),
                "follow the file to where it moved, or say what replaced it",
            ));
        }
    }
    out
}

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
