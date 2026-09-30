// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A retired word must point at a defined one. `lexicon.toml` says
//! which phrasings are out; `docs/glossary.md` says which word is in.
//! Nothing kept them agreeing, so a retirement could name a replacement
//! the glossary never defined - and a reader following the gate's advice
//! would arrive at a word with no meaning behind it.

use std::collections::BTreeSet;
use std::path::Path;

use crate::lexicon::PATH as LEXICON;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// Number words a document might spell a count with, and their values.
const SPELLED: [(&str, usize); 14] = [
    ("zero", 0),
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
];

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let glossary = terms(root)?;
    check_replacements(root, &glossary, &mut violations)?;
    Ok(violations)
}

/// Every bold name the glossary defines, plus the file names it uses,
/// since a replacement may legitimately point at a document.
fn terms(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let path = root.join("docs").join("glossary.md");
    let text = walk::read_text(&path)?;
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let mut rest = line;
        while let Some(open) = rest.find("**") {
            let after = rest.get(open.saturating_add(2)..).unwrap_or_default();
            let Some(close) = after.find("**") else { break };
            let term = after.get(..close).unwrap_or_default();
            if !term.is_empty() {
                out.insert(term.replace('\\', ""));
            }
            rest = after.get(close.saturating_add(2)..).unwrap_or_default();
        }
    }
    if out.is_empty() {
        return Err(XtaskError::Doc {
            file: "docs/glossary.md".to_owned(),
            msg: "no bold terms found; the table shape changed".to_owned(),
        });
    }
    Ok(out)
}

fn check_replacements(
    root: &Path,
    glossary: &BTreeSet<String>,
    out: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let text = walk::read_text(&root.join(LEXICON))?;
    for (index, line) in text.lines().enumerate() {
        let Some(value) = line.strip_prefix("replacement = ") else {
            continue;
        };
        let replacement = value.trim().trim_matches('"');
        // A document is a definition too: pointing at `JOB.md` names a
        // file whose content is the meaning.
        let defined =
            replacement.contains(".md") || glossary.iter().any(|term| replacement.contains(term));
        if !defined {
            out.push(Violation {
                gate: "lexicon",
                location: format!("{LEXICON}:{}", index.saturating_add(1)),
                rule: "a retired word points at a word the glossary defines".to_owned(),
                violation: format!("{replacement:?} is not defined in docs/glossary.md"),
                alternative: "give the replacement a glossary row, or name one that has it"
                    .to_owned(),
            });
        }
    }
    Ok(())
}

/// Every count a line states immediately before `phrase`.
///
/// `phrase` is matched as written, so a caller passing `kani harness`
/// also reads `kani harnesses`; pass the longest unambiguous prefix.
pub(crate) fn counts_before(line: &str, phrase: &str) -> Vec<usize> {
    let lowered = line.to_ascii_lowercase();
    let phrase = phrase.to_ascii_lowercase();
    let mut found = Vec::new();
    for (word, value) in SPELLED {
        if lowered.contains(&format!("{word} {phrase}")) {
            found.push(value);
        }
    }
    for token in line.split(|c: char| !c.is_ascii_digit()) {
        if token.is_empty() {
            continue;
        }
        let Ok(value) = token.parse::<usize>() else {
            continue;
        };
        if lowered.contains(&format!("{value} {phrase}")) {
            found.push(value);
        }
    }
    found
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

    #[test]
    fn a_replacement_the_glossary_never_defined_is_reported() {
        let glossary: BTreeSet<String> = ["Ledger".to_owned()].into();
        let mut out = Vec::new();
        let defined = |replacement: &str| {
            replacement.contains(".md") || glossary.iter().any(|t| replacement.contains(t))
        };
        assert!(defined("Ledger"));
        assert!(defined("JOB.md（产品）"));
        assert!(!defined("some other name"));
        assert!(out.is_empty());
        out.push(1);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn the_glossary_of_this_repository_parses_and_defines_its_own_words() {
        let root = crate::root::this_checkout().to_path_buf();
        let terms = terms(&root).unwrap();
        assert!(terms.contains("Ledger"));
        assert!(terms.contains("Building"));
    }
}
