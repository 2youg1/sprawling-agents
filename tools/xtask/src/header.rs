// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! MPL-2.0 header gate: every `.rs` in the repo opens with the exact
//! Exhibit A notice, then the copyright line.
//!
//! Markdown is out of this gate's scope by ruling: a skill document
//! carries its own licence in its frontmatter and in `skills/LICENSES.md`,
//! and an MPL notice on one of the MIT adaptations would misstate its
//! terms.
//!
//! The notice is what the licence asks for. The copyright line is not:
//! Mozilla's own FAQ answers "what do I have to do" with the notice alone
//! and says a name "is not necessary" (MPL 2.0 FAQ, Q4). It ships here
//! because the person who owns this tree chose to be named in it, and it
//! is gated for the reason every other convention in this repo is gated -
//! a line nothing checks is a line the next new file will not carry.
//!
//! The notice takes the first three rows because MPL 2.0 section 3.4
//! forbids altering the substance of a licence notice, so it is the part
//! that must stay quotable and verbatim, and nothing may split it. The
//! copyright line follows immediately, with no blank comment row between
//! them: the four rows are one head, and a reader who has finished the
//! third row is already at the name.
//!
//! The year is the year of first publication and does not advance with the
//! calendar - a gate that demanded the current year would turn every
//! January red across every file at once, which is a chore rather than a
//! fact about the work.
//!
//! "and the sprawling contributors" names people who do not exist yet on
//! purpose. Contributors hold copyright in what they write and grant it
//! downstream themselves under section 2.1, so the clause transfers
//! nothing; what it buys is that the first outside contribution does not
//! oblige anyone to rewrite every header in the tree. Exhibit A allows
//! "additional accurate notices of copyright ownership", and a standing
//! class is accurate in a way an enumerated list is not - Mozilla dropped
//! the per-file contributor list in the 1.1 to 2.0 upgrade because it was
//! "neither a complete nor accurate list" and a source of merge conflicts.
//!
//! RefRain and kusanagi carry the same four rows with their own project
//! name. Anyone changing the shape here changes it in all three.
//!
//! **The notice appears once, and the gate reads the whole file to say
//! so.** Comparing only the first four rows would let a second copy of
//! the header live further down and stay green. A duplicated notice is
//! how a file records that it was assembled from two files, and the
//! module documentation it splits is what the next reader needs.

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The MPL-2.0 notice and the copyright line, without the comment leader
/// a language puts in front of them; `notice` adds it.
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
    Lean,
}

/// The four rows as a file in that language spells them.
pub(crate) fn notice(leader: Leader) -> [String; 4] {
    let mark = match leader {
        Leader::Rust => "//",
        Leader::Lean => "--",
    };
    NOTICE.map(|row| format!("{mark} {row}"))
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let expected = notice(Leader::Rust);
    let mut violations = Vec::new();
    for file in walk::files_with_ext(root, &["rs"])? {
        let rel = walk::rel(root, &file);
        if walk::in_isolation_zone(&rel) {
            continue;
        }
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
                rule: "every .rs file carries the MPL-2.0 notice and the copyright line".to_owned(),
                violation: "the first four lines differ from the header".to_owned(),
                alternative: "prepend the exact 4-line header; see any existing module".to_owned(),
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

/// The one-based line where the notice begins a second time, if it
/// does. The first row is enough to find it: the opening comparison has
/// already established that this file starts with the whole notice.
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
    use super::{Leader, NOTICE, check, notice, repeated};

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
        let head: String = NOTICE.iter().map(|row| format!("// {row}\n")).collect();
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
