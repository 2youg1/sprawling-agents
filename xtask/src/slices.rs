// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Gate: the session-slice path has one writer and no readers
//! (memory-SPEC 8-24).
//!
//! A slice is a disposable projection: the Ledger stays the only
//! authority, and the product answers every question from the Ledger, so
//! no module outside `memory::sessions` may name where a slice lies. A
//! second reference would be the first step towards a reader, and a
//! reader of a projection that is deleted and rebuilt is a decision
//! resting on bytes nothing promises.
//!
//! The judged surface is every `.rs` under the repository except the two
//! files that share the fact: `kernel::layout` declares the path and
//! `memory::sessions` writes it. Three tokens are checked, each of them
//! how a reference is actually spelled: the directory constant
//! `SESSIONS_DIR`, the layout method `session_slice`, and the inverse
//! `of_ledger` a writer needs to find the city root. The bare string
//! `"sessions"` is checked too, because a call site that joined the path
//! by hand would spell nothing else.
//!
//! **The limit is recorded rather than hidden**: a path spelled through
//! an interposed alias (`let what = "sess"; what.to_owned() + "ions"`)
//! passes. This gate is a tripwire for the ordinary mistake, not a proof;
//! the sentence it holds is stated in the writer's module documentation.

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The files the slice path is allowed to name, and why: the layout that
/// derives it, the writer that alone calls the derivation, and the
/// writer's own fixtures.
const SHARES_THE_FACT: [&str; 3] = [
    "crates/kernel/src/layout.rs",
    "crates/memory/src/sessions.rs",
    "crates/memory/src/sessions/tests.rs",
];

/// The identifiers a reference is spelled with.
const PATH_WORDS: [&str; 3] = ["SESSIONS_DIR", "session_slice", "of_ledger"];

/// The literal a hand-joined path would carry.
const PATH_LITERAL: &str = "\"sessions\"";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for file in walk::files_with_ext(root, &["rs"])? {
        let rel = walk::rel(root, &file);
        if SHARES_THE_FACT.contains(&rel.as_str()) || rel == "xtask/src/slices.rs" {
            continue;
        }
        let text = walk::read_text(&file)?;
        for (index, line) in text.lines().enumerate() {
            for word in PATH_WORDS {
                if line.contains(word) {
                    violations.push(finding(&rel, index.saturating_add(1), word));
                }
            }
            if line.contains(PATH_LITERAL) {
                violations.push(finding(&rel, index.saturating_add(1), PATH_LITERAL));
            }
        }
    }
    Ok(violations)
}

fn finding(rel: &str, line: usize, word: &str) -> Violation {
    Violation {
        gate: "slices",
        location: format!("{rel}:{line}"),
        rule: "only `memory::sessions` names the session-slice path".to_owned(),
        violation: format!("this line names the path with `{word}`"),
        alternative: "ask the Ledger, or file the record through the writer; a slice is a \
                      projection nothing in the product reads"
            .to_owned(),
    }
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

    /// The gate is checked against the tree it guards, so a sentence that
    /// drifts is caught by running it rather than by reading it.
    #[test]
    fn only_the_writer_references_the_slice_path() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask is a workspace member");
        let violations = check(root).expect("the tree can be walked");
        assert!(
            violations.is_empty(),
            "the gate would red on the tree it ships with: {violations:?}"
        );
    }

    /// And it bites: a module that is not the writer, naming the path,
    /// is a finding. The fixture is a temporary tree, so the proof does
    /// not depend on the repository staying clean.
    #[test]
    fn a_second_module_naming_the_path_is_a_finding() {
        let root = std::env::temp_dir().join(format!("xtask-slices-{}", std::process::id()));
        let src = root.join("crates").join("elsewhere").join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("reader.rs"),
            "fn p() { let _ = \"sessions\"; }\nfn q() { /* session_slice */ }\n",
        )
        .unwrap();
        let violations = check(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(violations.len(), 2, "{violations:?}");
        assert!(
            violations.iter().all(|finding| finding
                .location
                .starts_with("crates/elsewhere/src/reader.rs:")),
            "{violations:?}"
        );
    }
}
