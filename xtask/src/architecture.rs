// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading `ARCHITECTURE.md` by section, for every gate that judges it.
//!
//! **A table is recognised by the section it sits in, not by its shape.**
//! `depmap` read the seam list as "any pipe row with four cells whose
//! second cell is a `crates/**.rs` path", which is a description of a
//! table rather than of a place: a fifth four-column table added anywhere
//! in this document would have quietly widened the set of files allowed
//! to declare a `pub trait`, and nothing would have said so. `modmap`
//! carried the module map's section number in one of its two readers and
//! not in the other.
//!
//! So the document's structure has one reader, here. A section opens
//! with a `## <number> <title>` heading and closes at the next one;
//! `###` subheadings belong to the section above them, which is what the
//! module map is written as. Lines arrive with the number they have in
//! the whole document, so a gate reporting `ARCHITECTURE.md:412` still
//! names the line a person opens.

/// The document every gate in this module's field of view judges.
pub(crate) const PATH: &str = "ARCHITECTURE.md";

/// One line of a section, and where it sits in the whole document.
pub(crate) struct Numbered<'t> {
    pub(crate) line: usize,
    pub(crate) text: &'t str,
}

/// Every line under the `## <number>` heading, the heading excluded, or
/// nothing when this document has no such section.
pub(crate) fn section(text: &str, number: u32) -> Option<Vec<Numbered<'_>>> {
    let opening = format!("## {number} ");
    let mut lines = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with("## ") {
            if inside {
                return Some(lines);
            }
            inside = line.starts_with(&opening);
            continue;
        }
        if inside {
            lines.push(Numbered {
                line: index.saturating_add(1),
                text: line,
            });
        }
    }
    inside.then_some(lines)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::section;

    const DOC: &str = "\
# Title
## 3 Units
| a | b |
## 4 Seams
| seam | crates/kernel/src/ledger.rs |
### An aside that belongs to section 4
| seam | crates/browser/src/port.rs |
## 5 Next
| seam | crates/kernel/src/nowhere.rs |
";

    #[test]
    fn a_section_reaches_its_subheadings_and_stops_at_the_next_section() {
        let found = section(DOC, 4).unwrap();
        let body: Vec<&str> = found.iter().map(|line| line.text).collect();
        assert_eq!(body.len(), 3, "{body:?}");
        assert!(body.iter().any(|line| line.contains("port.rs")));
        assert!(
            !body.iter().any(|line| line.contains("nowhere.rs")),
            "the reading ran past the end of its section"
        );
    }

    /// The numbers are the document's, so a gate that reports a location
    /// names a line somebody can open.
    #[test]
    fn a_line_keeps_the_number_it_has_in_the_document() {
        let found = section(DOC, 4).unwrap();
        assert_eq!(found[0].line, 5);
    }

    /// A section that is not there is not an empty section: a gate whose
    /// authority has moved must say so rather than judge against nothing.
    #[test]
    fn a_missing_section_is_absent_rather_than_empty() {
        assert!(section(DOC, 9).is_none());
        assert!(section(DOC, 3).is_some());
    }
}
