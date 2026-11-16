// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) const PATH: &str = "docs/ARCHITECTURE.md";

pub(crate) fn section(text: &str, number: u32) -> Option<Vec<&str>> {
    let opening = format!("## {number} ");
    let mut lines = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("## ") {
            if inside {
                return Some(lines);
            }
            inside = line.starts_with(&opening);
            continue;
        }
        if inside {
            lines.push(line);
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
| seam | Sprawling/kernel/src/ledger.rs |
### An aside that belongs to section 4
| seam | Sprawling/browser/src/port.rs |
## 5 Next
| seam | Sprawling/kernel/src/nowhere.rs |
";

    #[test]
    fn a_section_reaches_its_subheadings_and_stops_at_the_next_section() {
        let body = section(DOC, 4).unwrap();
        assert_eq!(body.len(), 3, "{body:?}");
        assert!(body.iter().any(|line| line.contains("port.rs")));
        assert!(
            !body.iter().any(|line| line.contains("nowhere.rs")),
            "the reading ran past the end of its section"
        );
    }

    #[test]
    fn a_missing_section_is_absent_rather_than_empty() {
        assert!(section(DOC, 9).is_none());
        assert!(section(DOC, 3).is_some());
    }
}
