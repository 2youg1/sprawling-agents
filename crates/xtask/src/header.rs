// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Require [`NOTICE`] exactly once at the top of every `.rs` and `.zig`.
//! The first three rows are the MPL notice; the owner's copyright row is
//! fourth. Its year is first publication, not the current year. Markdown
//! keeps licence metadata in frontmatter and `crates/city/skills/LICENSES.md`.
//! RefRain and kusanagi use the same shape with their own project names.
//! A repeated notice marks a file assembled from two modules.

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The four header rows without a language's comment leader.
pub(crate) const NOTICE: [&str; 4] = [
    "This Source Code Form is subject to the terms of the Mozilla Public",
    "License, v. 2.0. If a copy of the MPL was not distributed with this",
    "file, You can obtain one at https://mozilla.org/MPL/2.0/.",
    "Copyright (c) 2026 2youg1 and the sprawling contributors",
];

/// The line comment of a language the header is written in.
#[derive(Clone, Copy)]
pub(crate) enum Leader {
    Rust,
    Zig,
    Lean,
}

/// File extensions accepted by the gate, including the Zig FFI condition.
const JUDGED: [(Leader, &str); 2] = [(Leader::Rust, "rs"), (Leader::Zig, "zig")];

/// The four rows as a file in that language spells them.
pub(crate) fn notice(leader: Leader) -> [String; 4] {
    let mark = match leader {
        Leader::Rust | Leader::Zig => "//",
        Leader::Lean => "--",
    };
    NOTICE.map(|row| format!("{mark} {row}"))
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for file in walk::files_with_ext(root, &JUDGED.map(|(_, extension)| extension))? {
        let rel = walk::rel(root, &file);
        let Some(leader) = leader_of(&file) else {
            continue;
        };
        if walk::in_isolation_zone(&rel) {
            continue;
        }
        let expected = notice(leader);
        let text = walk::read_text(&file)?;
        let rows: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        let opens = expected
            .iter()
            .zip(rows.iter())
            .all(|(want, found)| want == found);
        if !opens {
            violations.push(Violation {
                gate: "header",
                location: rel.clone(),
                rule: "every .rs and .zig file carries the MPL-2.0 notice and the copyright line"
                    .to_owned(),
                violation: "the first four lines differ from the header".to_owned(),
                alternative: "prepend the exact four lines in `header::NOTICE` with the language's comment leader"
                    .to_owned(),
            });
            continue;
        }
        if let Some(again) = repeated(&rows, &expected) {
            violations.push(Violation {
                gate: "header",
                location: format!("{rel}:{again}"),
                rule: "the notice appears once in a file, at the top of it".to_owned(),
                violation: "a second copy of the header starts here".to_owned(),
                alternative: "delete this copy, and read what sits around it: a file with two \
                              headers was assembled from two files, and the documentation \
                              between them may describe the other one"
                    .to_owned(),
            });
        }
    }
    Ok(violations)
}

/// The language a judged file is written in, by its extension.
fn leader_of(file: &Path) -> Option<Leader> {
    let extension = file.extension()?.to_str()?;
    JUDGED
        .iter()
        .find(|(_, judged)| *judged == extension)
        .map(|(leader, _)| *leader)
}

/// The one-based line of a repeated notice, if present.
fn repeated(rows: &[&str], expected: &[String; 4]) -> Option<usize> {
    let first = expected.first()?;
    rows.iter()
        .enumerate()
        .skip(1)
        .find(|(_, row)| **row == first.as_str())
        .map(|(index, _)| index.saturating_add(1))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::{Leader, check, notice, repeated};

    #[test]
    fn this_file_carries_the_header() {
        let text = include_str!("header.rs");
        let mut lines = text.lines();
        for want in notice(Leader::Rust) {
            assert_eq!(lines.next(), Some(want.as_str()));
        }
    }

    #[test]
    fn a_second_copy_is_found_wherever_it_sits() {
        let expected = notice(Leader::Rust);
        let once: Vec<&str> = expected
            .iter()
            .map(String::as_str)
            .chain(["", "//! a module"])
            .collect();
        assert_eq!(repeated(&once, &expected), None);
        let twice: Vec<&str> = once
            .iter()
            .copied()
            .chain(expected.iter().map(String::as_str))
            .collect();
        assert_eq!(repeated(&twice, &expected), Some(7));
    }

    /// A Zig file is held to the same four rows as a Rust file, behind
    /// the same `//`; the one without them is named and the others are
    /// not.
    #[test]
    fn a_zig_file_without_the_header_is_named() {
        let root = std::env::temp_dir().join(format!("xtask-header-zig-{}", std::process::id()));
        let head: String = notice(Leader::Zig)
            .iter()
            .map(|row| format!("{row}\n"))
            .collect();
        crate::root::fixture::write(&root, "kept.rs", &format!("{head}\nfn kept() {{}}\n"));
        crate::root::fixture::write(&root, "leaf/headed.zig", &format!("{head}\nconst a = 0;\n"));
        crate::root::fixture::write(&root, "leaf/bare.zig", "const std = @import(\"std\");\n");
        let found = check(&root).map(|all| all.into_iter().map(|v| v.location).collect::<Vec<_>>());
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            found.map_err(|err| err.to_string()),
            Ok(vec!["leaf/bare.zig".to_owned()])
        );
    }
}
