// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod context;
mod link;

use context::working_record;
use link::{Spellings, link_targets, resolve};

const SCAFFOLDING: [&str; 1] = ["local/"];

const HOME_SHAPES: [&str; 7] = [
    ":\\users\\",
    ":/users/",
    ":\\home\\",
    ":/home/",
    "/home/",
    "/users/",
    "/root/",
];

const DETECTORS: [&str; 5] = [
    "Tools/xtask/src/release.rs",
    "Tools/xtask/src/release/context.rs",
    "Tools/xtask/src/release/tests.rs",
    "Tools/xtask/Spec.lean",
    "AGENTS.md",
];

pub(crate) const CITED_EXTENSIONS: [&str; 8] = [
    ".md", ".rs", ".toml", ".html", ".css", ".json", ".jsonl", ".lean",
];

fn directory_names(published: &[String]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for rel in published {
        let mut parts: Vec<&str> = rel.split('/').collect();
        parts.pop();
        for part in parts {
            names.insert(part.to_owned());
        }
    }
    names.insert("local".to_owned());
    names.insert("target".to_owned());
    names
}

pub(crate) fn path_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in line.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-' | '/') {
            current.push(ch);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[must_use]
pub(crate) fn outside_the_tree(line: &str, dirs: &BTreeSet<String>) -> Option<String> {
    path_tokens(line).into_iter().find(|token| {
        let trimmed = token.trim_end_matches(['.', '-']);
        if !CITED_EXTENSIONS.iter().any(|ext| trimmed.ends_with(ext)) {
            return false;
        }
        let Some((first, _)) = trimmed.split_once('/') else {
            return false;
        };
        if !first.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
            return false;
        }
        !dirs.contains(first)
    })
}

pub(crate) fn is_prose(rel: &str, line: &str) -> bool {
    if rel.ends_with(".md") || rel.ends_with(".html") || rel.ends_with(".lean") {
        return true;
    }
    rel.ends_with(".rs") && line.trim_start().starts_with("//")
}

#[must_use]
pub(crate) fn machine_path(line: &str) -> Option<String> {
    let lowered = line.to_lowercase();
    let at = HOME_SHAPES
        .iter()
        .filter_map(|shape| lowered.find(shape))
        .min()?;
    let chars_before = lowered.char_indices().take_while(|(i, _)| *i < at).count();
    Some(lowered.chars().skip(chars_before).take(20).collect())
}

#[must_use]
pub(crate) fn is_scaffolding(rel: &str) -> bool {
    SCAFFOLDING.iter().any(|prefix| {
        let bare = prefix.trim_end_matches('/');
        rel == *prefix || rel == bare || rel.starts_with(prefix)
    })
}

pub(crate) fn published(root: &Path) -> Result<Vec<String>, XtaskError> {
    Ok(walk::files(root)?
        .iter()
        .map(|path| walk::rel(root, path))
        .filter(|rel| !is_scaffolding(rel))
        .collect())
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let listed = published(root)?;
    let mut dirs = directory_names(&listed);
    for row in crate::product_roots::load(root)?.legacy {
        dirs.insert(row.path);
    }
    let kept: BTreeSet<String> = listed.into_iter().collect();
    let spellings = Spellings::of(&kept);
    for rel in &kept {
        let full = root.join(rel);
        let Ok(text) = std::fs::read_to_string(&full) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            if DETECTORS.contains(&rel.as_str()) {
                break;
            }
            if let Some(found) = machine_path(line) {
                violations.push(Violation {
                    gate: "release",
                    location: format!("{rel}:{}", number.saturating_add(1)),
                    rule: "a published file may not carry a path that exists only on the \
                           machine that wrote it"
                        .to_owned(),
                    violation: format!("names `{found}`"),
                    alternative: "write the path relative to the city or the repository, or \
                                  use a placeholder a reader can substitute"
                        .to_owned(),
                });
            }
            if let Some(shape) = working_record(line) {
                violations.push(Violation {
                    gate: "release",
                    location: format!("{rel}:{}", number.saturating_add(1)),
                    rule: "a published file states the decision, never the occasion: not one \
                           machine, not one sitting, not a plan's label, not a message to a \
                           person"
                        .to_owned(),
                    violation: format!("says `{shape}`"),
                    alternative: "report a measurement against the machine class it came \
                                  from; give a ruling and its reason without the sitting \
                                  that produced it; write the reason or the criterion a \
                                  plan's label stood for, or cite the SPEC section that \
                                  holds it; and where a question is open, say what about \
                                  the interface is undecided rather than who is to decide it"
                        .to_owned(),
                });
            }
            if is_prose(rel, line)
                && let Some(cited) = outside_the_tree(line, &dirs)
            {
                violations.push(Violation {
                    gate: "release",
                    location: format!("{rel}:{}", number.saturating_add(1)),
                    rule: "a published document may not cite a path anchored outside this \
                           repository"
                        .to_owned(),
                    violation: format!("`{cited}` starts at a directory this tree does not have"),
                    alternative: "decide which of the two is misfiled: move this file to \
                                  local/ if it is one machine's working note, bring the \
                                  cited document into the tree if it is an authority, or \
                                  write down here what it says. A reader who clones this \
                                  repository has nowhere else to look"
                        .to_owned(),
                });
            }
        }
        if !rel.ends_with(".md") {
            continue;
        }
        if text.contains("local/") && !DETECTORS.contains(&rel.as_str()) {
            violations.push(Violation {
                gate: "release",
                location: rel.clone(),
                rule: "a published document may not send a reader into the isolation zone, \
                       which exists on one machine only"
                    .to_owned(),
                violation: "names `local/`".to_owned(),
                alternative: "say the thing itself here, or drop the sentence: a reader who \
                              cannot follow it is worse off than one who never saw it"
                    .to_owned(),
            });
        }
        for target in link_targets(&text) {
            if target.starts_with("http") || target.starts_with('#') {
                continue;
            }
            let cleaned = target.split('#').next().unwrap_or(&target).to_owned();
            if cleaned.is_empty() {
                continue;
            }
            let pointed = resolve(rel, &cleaned);
            if is_scaffolding(&pointed) {
                violations.push(Violation {
                    gate: "release",
                    location: format!("{rel} -> {target}"),
                    rule: "a product document may not depend on scaffolding, because the \
                           link is dead in the public artefact"
                        .to_owned(),
                    violation: format!("{pointed} is filtered out of the public tree"),
                    alternative: "move the sentence into the product document, or drop it: a \
                                  link a reader cannot follow is worse than no link"
                        .to_owned(),
                });
            } else if let Some(on_disk) = spellings.miscased(&pointed) {
                violations.push(Violation {
                    gate: "release",
                    location: format!("{rel} -> {target}"),
                    rule: "a link spells the name the tree carries, case included".to_owned(),
                    violation: format!("the tree has `{on_disk}`, not `{pointed}`"),
                    alternative: format!(
                        "write the link as `{on_disk}`, or rename the file and move every \
                         reader with it: Linux opens one spelling only"
                    ),
                });
            }
        }
    }
    Ok(violations)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
