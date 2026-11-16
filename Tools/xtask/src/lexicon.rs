// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One concept, one name. Data face is `Tools/xtask/lexicon.toml`; exempt
//! with `lexicon-ok: <reason>` on the line or the line above (redline C6).

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// Retired words; `vocabulary` reads the same path.
pub(crate) const PATH: &str = "Tools/xtask/lexicon.toml";

const EXEMPT_MARK: &str = "lexicon-ok:";
const LANG: &str = concat!(crate::walk::client_src!(), "/lang.json");

#[derive(serde::Deserialize)]
struct Data {
    entry: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    banned: String,
    replacement: String,
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let data = load(root)?;
    let mut violations = Vec::new();
    violations.extend(crate::vocabulary::check(root)?);
    for rel in sources(root)? {
        let text = walk::read_text(&root.join(&rel))?;
        scan(&rel, &text, &data.entry, &mut violations);
    }
    let text = walk::read_text(&root.join(LANG))?;
    scan(LANG, &text, &data.entry, &mut violations);
    Ok(violations)
}

fn sources(root: &Path) -> Result<Vec<String>, XtaskError> {
    Ok(walk::files_with_ext(root, &["md", "rs", "lean"])?
        .iter()
        .map(|file| walk::rel(root, file))
        .filter(|rel| !walk::in_isolation_zone(rel))
        .collect())
}

fn load(root: &Path) -> Result<Data, XtaskError> {
    let text = walk::read_text(&root.join(PATH))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: PATH.to_owned(),
        msg: err.to_string(),
    })
}

fn scan(location: &str, text: &str, entries: &[Entry], out: &mut Vec<Violation>) {
    let mut previous_exempts = false;
    for (index, line) in text.lines().enumerate() {
        let marked = line.contains(EXEMPT_MARK);
        let exempt = marked || previous_exempts;
        if !exempt {
            for entry in entries {
                if line.contains(&entry.banned) {
                    out.push(Violation {
                        gate: "lexicon",
                        location: format!("{location}:{}", index.saturating_add(1)),
                        rule: "the vocabulary authority is docs/glossary.md".to_owned(),
                        violation: format!("banned term {:?}", entry.banned),
                        alternative: format!(
                            "use {}; if this is another concept, narrow the scanner to that exact context instead of adding a waiver",
                            entry.replacement
                        ),
                    });
                }
            }
        }
        previous_exempts = marked;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry> {
        vec![Entry {
            banned: "banned-term".to_owned(),
            replacement: "Ledger".to_owned(),
        }]
    }

    #[test]
    fn hit_is_reported_with_line_number() {
        let mut out = Vec::new();
        scan(
            "doc.md",
            "ok\nuses banned-term here\n",
            &entries(),
            &mut out,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].location, "doc.md:2");
    }

    #[test]
    fn a_lean_specification_is_read_beside_documents_and_rust() {
        let root = std::env::temp_dir().join(format!("lexicon-sources-{}", std::process::id()));
        for rel in ["a.md", "b.rs", "crates/x/Spec.lean", "d.txt", "local/e.md"] {
            crate::root::fixture::write(&root, rel, "");
        }
        let read = sources(&root).map_err(|err| err.to_string());
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            read,
            Ok(vec![
                "a.md".to_owned(),
                "b.rs".to_owned(),
                "crates/x/Spec.lean".to_owned()
            ])
        );
    }

    #[test]
    fn same_line_and_previous_line_marks_exempt() {
        let mut out = Vec::new();
        scan(
            "doc.md",
            "banned-term lexicon-ok: quoting the old name\n",
            &entries(),
            &mut out,
        );
        scan(
            "doc.md",
            "lexicon-ok: next line quotes it\nbanned-term\n",
            &entries(),
            &mut out,
        );
        assert!(out.is_empty());
    }
}
