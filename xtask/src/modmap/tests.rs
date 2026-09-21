// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

const MODULE_ROW: &str = "| kernel::gate | crates/kernel/src/gate.rs | five gates | \
     decision | S2 | planned | kernel-SPEC.md#8-27 |";
const SEAM_ROW: &str = "| kernel::ledger | crates/kernel/src/ledger.rs | memory jsonl | citysim |";

/// A body of table lines, as the module map's own section reads.
fn numbered(text: &str) -> Vec<Numbered<'_>> {
    text.lines()
        .enumerate()
        .map(|(index, line)| Numbered {
            line: index.saturating_add(1),
            text: line,
        })
        .collect()
}

#[test]
fn module_row_parses_and_seam_row_is_ignored() {
    let mut v = Vec::new();
    let rows = parse_rows(&numbered(&format!("{MODULE_ROW}\n{SEAM_ROW}\n")), &mut v);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, "crates/kernel/src/gate.rs");
    assert!(v.is_empty());
}

#[test]
fn bad_status_and_duplicate_are_violations() {
    let bad = "| kernel::gate | crates/kernel/src/gate.rs | x | 8.2 | S2 | done | s.md#8-1 |";
    let mut v = Vec::new();
    let rows = parse_rows(
        &numbered(&format!("{bad}\n{MODULE_ROW}\n{MODULE_ROW}\n")),
        &mut v,
    );
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
    let text = format!("## {MODULE_SECTION} Module map\n\n### kernel (9)\n\n{MODULE_ROW}\n");
    let map = module_map(&text).unwrap();
    let mut ignored = Vec::new();
    let rows = parse_rows(&map, &mut ignored);
    let mut violations = Vec::new();
    check_counts(&map, &rows, &mut violations);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].alternative.contains("kernel (1)"));
}

/// A table row written outside the module map is not a module row: the
/// section it sits in is what makes it one.
#[test]
fn a_row_outside_the_module_map_is_not_read() {
    let text = format!("## 6 On disk\n{MODULE_ROW}\n## {MODULE_SECTION} Module map\n");
    let map = module_map(&text).unwrap();
    let mut ignored = Vec::new();
    assert!(parse_rows(&map, &mut ignored).is_empty());
}

#[test]
fn index_name_requires_registered_children() {
    let dirs: BTreeSet<String> = ["crates/runtime/src/tools".to_owned()].into();
    assert!(is_index_name("crates/runtime/src/tools.rs", &dirs));
    assert!(is_index_name("crates/kernel/src/lib.rs", &dirs));
    assert!(!is_index_name("crates/kernel/src/util.rs", &dirs));
}
