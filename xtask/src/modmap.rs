// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Module-table gate (redline C4). The table in ARCHITECTURE.md section 6 is a
//! closed list: every `.rs` under `crates/*/src` is either a registered module,
//! a `lib.rs`, or a pure index file. Status coherence is checked both ways —
//! a file that exists while its row still says planned means the builder skipped
//! the "flip the status" leg of the completion evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

const ARCH: &str = "ARCHITECTURE.md";
/// The status column, in the language the table is written in. It moved
/// from Chinese to English when the module table became part of what the
/// repository publishes: a reader of `ARCHITECTURE.md` should not have to
/// read a second language to find out whether a module exists.
const STATUS_PLANNED: &str = "planned";
const STATUSES: [&str; 4] = [STATUS_PLANNED, "building", "built", "frozen"];

/// Line prefixes allowed in `lib.rs` and pure index files (single-line
/// declarations only; a style constraint recorded in ARCHITECTURE.md section 5).
const INDEX_PREFIXES: [&str; 8] = [
    "//",
    "#![",
    "#[",
    "pub mod ",
    "mod ",
    "pub use ",
    "pub(crate) use ",
    "use ",
];

struct Row {
    module: String,
    path: String,
    shape: String,
    status: String,
    spec: String,
    line: usize,
}

/// One module's claim about where its interface is specified: the raw
/// seventh cell, with the row it came from.
///
/// Handed to `specalign`, which is the gate that owns "a SPEC says what
/// the code says". This parser stays the module table's only reader
/// (xtask-SPEC.md section 8-10), so a column change breaks one place.
pub(crate) struct Anchor {
    pub(crate) module: String,
    pub(crate) spec: String,
    pub(crate) line: usize,
}

/// Every registered module's `Spec` cell, in table order.
pub(crate) fn anchors(root: &Path) -> Result<Vec<Anchor>, XtaskError> {
    let text = walk::read_text(&root.join(ARCH))?;
    let mut ignored = Vec::new();
    Ok(parse_rows(&text, &mut ignored)
        .into_iter()
        .map(|row| Anchor {
            module: row.module,
            spec: row.spec,
            line: row.line,
        })
        .collect())
}

/// The shape each registered module states, by file path.
///
/// Read by `length`, which does not measure a module whose shape is
/// `data`. It comes from this parser rather than a second one: the
/// module table has one reader, so a change to its columns breaks one
/// place.
pub(crate) fn shapes(root: &Path) -> Result<BTreeMap<String, String>, XtaskError> {
    let text = walk::read_text(&root.join(ARCH))?;
    let mut ignored = Vec::new();
    Ok(parse_rows(&text, &mut ignored)
        .into_iter()
        .map(|row| (row.path, row.shape))
        .collect())
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let text = walk::read_text(&root.join(ARCH))?;
    let mut violations = Vec::new();
    let rows = parse_rows(&text, &mut violations);

    let table: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.path.as_str(), row)).collect();
    // Directories that hold registered modules; `<dir>.rs` is then an index file.
    let index_dirs: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| row.path.rsplit_once('/').map(|(dir, _)| dir.to_owned()))
        .collect();

    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    for file in walk::files_with_ext(&root.join("crates"), &["rs"])? {
        let rel = walk::rel(root, &file);
        if !rel.contains("/src/") {
            continue; // build.rs and friends are not modules
        }
        on_disk.insert(rel.clone());
        if let Some(row) = table.get(rel.as_str()) {
            if row.status == STATUS_PLANNED {
                violations.push(Violation {
                    gate: "modmap",
                    location: rel,
                    rule: "completion evidence includes flipping the module status \
                           (ARCHITECTURE.md section 12)"
                        .to_owned(),
                    violation: format!(
                        "file exists but its row (line {}) still says {STATUS_PLANNED}",
                        row.line
                    ),
                    alternative: "flip the status in the same change-set, or delete the file"
                        .to_owned(),
                });
            }
        } else if is_index_name(&rel, &index_dirs) {
            check_index_content(root, &rel, &mut violations)?;
        } else {
            violations.push(Violation {
                gate: "modmap",
                location: rel,
                rule: "the module table is a closed list (ARCHITECTURE.md section 6, C4)"
                    .to_owned(),
                violation: "file is not registered in the module table".to_owned(),
                alternative: "register the module (name/duty/stage/shape) in section 6 \
                              first, or delete the file"
                    .to_owned(),
            });
        }
    }

    check_counts(&text, &rows, &mut violations);

    for row in &rows {
        if row.status != STATUS_PLANNED && !on_disk.contains(&row.path) {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{ARCH}:{}", row.line),
                rule: "a non-planned status claims the file exists (section 6)".to_owned(),
                violation: format!("{} is marked {} but missing on disk", row.path, row.status),
                alternative: format!(
                    "create {} or set the row back to {STATUS_PLANNED}",
                    row.path
                ),
            });
        }
    }
    Ok(violations)
}

/// A module row has exactly seven data cells, a `crates/**.rs` path in cell 2,
/// `::` in cell 1, and a known status in cell 6. Seam-table rows (four cells)
/// and card checklists (not pipe rows) never match (xtask-SPEC.md section 10-2).
/// The seventh cell is `Spec`; it sits last so the shape and status
/// positions do not move.
fn parse_rows(text: &str, violations: &mut Vec<Violation>) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line_no = index.saturating_add(1);
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() != 9 {
            continue;
        }
        let (module, path, shape, status, spec) = match (
            cells.get(1),
            cells.get(2),
            cells.get(4),
            cells.get(6),
            cells.get(7),
        ) {
            (Some(m), Some(p), Some(h), Some(s), Some(k)) => (*m, *p, *h, *s, *k),
            _ => continue,
        };
        if !module.contains("::") || !path.starts_with("crates/") || !path.ends_with(".rs") {
            continue;
        }
        if !STATUSES.contains(&status) {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{ARCH}:{line_no}"),
                rule: "status is one of planned|building|built|frozen (the module-table \
                       column contract)"
                    .to_owned(),
                violation: format!("row for {path} has status {status:?}"),
                alternative: "use a value from the status enum".to_owned(),
            });
            continue;
        }
        if let Some(first) = seen.get(path) {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{ARCH}:{line_no}"),
                rule: "one module, one row (section 6)".to_owned(),
                violation: format!("{path} already registered at line {first}"),
                alternative: "merge the duplicate rows".to_owned(),
            });
            continue;
        }
        seen.insert(path.to_owned(), line_no);
        rows.push(Row {
            module: module.to_owned(),
            path: path.to_owned(),
            shape: shape.to_owned(),
            status: status.to_owned(),
            spec: spec.to_owned(),
            line: line_no,
        });
    }
    rows
}

/// The crate names a module-map subheading counts, each with the number it
/// claims: `### browser (6), protocol (5), bin (111)` yields three pairs.
///
/// A word immediately followed by ` (<digits>)` is a claim; anything else in
/// the heading is prose. Nothing else in this document writes that shape.
fn counted_crates(heading: &str) -> Vec<(String, usize)> {
    let mut pairs = Vec::new();
    let mut rest = heading;
    while let Some(open) = rest.find(" (") {
        let (before, after) = rest.split_at(open);
        let tail = after.get(2..).unwrap_or_default();
        let Some(close) = tail.find(')') else { break };
        let inside = tail.get(..close).unwrap_or_default();
        let name = before.rsplit([' ', ',']).next().unwrap_or_default();
        if let Ok(count) = inside.parse::<usize>()
            && !name.is_empty()
            && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        {
            pairs.push((name.to_owned(), count));
        }
        rest = tail.get(close..).unwrap_or_default();
    }
    pairs
}

/// Every number a module-map subheading states equals the rows under it.
///
/// A count a person maintains by hand is a count that has already gone
/// stale: nine of the thirteen were wrong when this assertion landed
/// (xtask-SPEC.md section 8-10). `desktop` is out of the parser's reach —
/// its rows carry `desktop/` paths, which `parse_rows` does not admit —
/// so its heading is not judged here.
fn check_counts(text: &str, rows: &[Row], violations: &mut Vec<Violation>) {
    let mut heading: Option<(usize, String)> = None;
    let mut in_map = false;
    let mut sections: Vec<(usize, usize, String)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_no = index.saturating_add(1);
        if line.starts_with("## ") {
            in_map = line.starts_with("## 12 ");
            if let Some((at, title)) = heading.take() {
                sections.push((at, line_no, title));
            }
            continue;
        }
        if !in_map {
            continue;
        }
        if let Some(title) = line.strip_prefix("### ") {
            if let Some((at, previous)) = heading.take() {
                sections.push((at, line_no, previous));
            }
            heading = Some((line_no, title.to_owned()));
        }
    }
    if let Some((at, title)) = heading {
        sections.push((at, usize::MAX, title));
    }

    for (start, end, title) in sections {
        for (name, claimed) in counted_crates(&title) {
            let prefix = format!("{name}::");
            let actual = rows
                .iter()
                .filter(|row| row.line > start && row.line < end && row.module.starts_with(&prefix))
                .count();
            if actual == 0 || actual == claimed {
                continue;
            }
            violations.push(Violation {
                gate: "modmap",
                location: format!("{ARCH}:{start}"),
                rule: "a subheading's count equals the rows under it \
                       (xtask-SPEC.md section 8-10)"
                    .to_owned(),
                violation: format!("heading says {name} ({claimed}), the table has {actual}"),
                alternative: format!("write {name} ({actual})"),
            });
        }
    }
}

fn is_index_name(rel: &str, index_dirs: &BTreeSet<String>) -> bool {
    if rel.ends_with("/lib.rs") {
        return true;
    }
    rel.strip_suffix(".rs")
        .is_some_and(|stem| index_dirs.contains(stem))
}

fn check_index_content(
    root: &Path,
    rel: &str,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let text = walk::read_text(&root.join(rel))?;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let allowed =
            line.is_empty() || INDEX_PREFIXES.iter().any(|prefix| line.starts_with(prefix));
        if !allowed {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{rel}:{}", index.saturating_add(1)),
                rule: "index files hold declarations only — comments, attributes, \
                       mod, use (ARCHITECTURE.md section 5)"
                    .to_owned(),
                violation: format!("logic line in an index file: {line:?}"),
                alternative: "move the logic into a registered module".to_owned(),
            });
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    const MODULE_ROW: &str = "| kernel::gate | crates/kernel/src/gate.rs | five gates | \
         decision | S2 | planned | kernel-SPEC.md#8-27 |";
    const SEAM_ROW: &str =
        "| kernel::ledger | crates/kernel/src/ledger.rs | memory jsonl | citysim |";

    #[test]
    fn module_row_parses_and_seam_row_is_ignored() {
        let mut v = Vec::new();
        let rows = parse_rows(&format!("{MODULE_ROW}\n{SEAM_ROW}\n"), &mut v);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "crates/kernel/src/gate.rs");
        assert!(v.is_empty());
    }

    #[test]
    fn bad_status_and_duplicate_are_violations() {
        let bad = "| kernel::gate | crates/kernel/src/gate.rs | x | 8.2 | S2 | done | s.md#8-1 |";
        let mut v = Vec::new();
        let rows = parse_rows(&format!("{bad}\n{MODULE_ROW}\n{MODULE_ROW}\n"), &mut v);
        assert_eq!(rows.len(), 1);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn a_heading_states_one_count_per_crate_it_lists() {
        let one = counted_crates("kernel (73) — every decision in the city");
        assert_eq!(one, vec![("kernel".to_owned(), 73)]);
        let three = counted_crates("browser (6), protocol (5), bin (111)");
        assert_eq!(three.len(), 3);
        assert_eq!(three.get(2), Some(&("bin".to_owned(), 111)));
        assert!(counted_crates("The performance register").is_empty());
    }

    #[test]
    fn a_stale_count_is_a_violation() {
        let text = format!("## 12 Module map\n\n### kernel (9)\n\n{MODULE_ROW}\n");
        let mut ignored = Vec::new();
        let rows = parse_rows(&text, &mut ignored);
        let mut violations = Vec::new();
        check_counts(&text, &rows, &mut violations);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].alternative.contains("kernel (1)"));
    }

    #[test]
    fn index_name_requires_registered_children() {
        let dirs: BTreeSet<String> = ["crates/runtime/src/tools".to_owned()].into();
        assert!(is_index_name("crates/runtime/src/tools.rs", &dirs));
        assert!(is_index_name("crates/kernel/src/lib.rs", &dirs));
        assert!(!is_index_name("crates/kernel/src/util.rs", &dirs));
    }
}
