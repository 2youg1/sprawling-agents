// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// One well-formed entry. TOML gives an inline table one line, so these are
/// assembled at compile time rather than wrapped.
const REGISTERED: &str = concat!(
    r#"{ name = "kernel::gate", file = "crates/kernel/src/gate.rs", "#,
    r#"owns = "five gates", shape = "decision", since = "S2", "#,
    r#"status = "planned", spec = "kernel-SPEC.md#8-27" }"#,
);

/// A module of a tool package, which the map carries for a reader.
const TOOL: &str = concat!(
    r#"{ name = "sim::clock", file = "tools/sim/src/clock.rs", "#,
    r#"owns = "the counted clock", shape = "value", since = "F1", "#,
    r#"status = "built", spec = "sim-SPEC.md#8-1" }"#,
);

const BAD_STATUS: &str = concat!(
    r#"{ name = "kernel::a", file = "crates/kernel/src/a.rs", "#,
    r#"owns = "x", shape = "decision", since = "S2", "#,
    r#"status = "done", spec = "s.md#8-1" }"#,
);

const EMPTY_DUTY: &str = concat!(
    r#"{ name = "kernel::b", file = "crates/kernel/src/b.rs", "#,
    r#"owns = "   ", shape = "decision", since = "S2", "#,
    r#"status = "built", spec = "s.md#8-1" }"#,
);

fn map_of(entries: &[&str]) -> Map {
    let body = entries.join(",\n  ");
    toml::from_str(&format!("module = [\n  {body},\n]\n")).expect("a module map")
}

/// The product packages the entries above are judged against.
fn product() -> [Member; 1] {
    [Member {
        package: "kernel".to_owned(),
        lib: Some("kernel".to_owned()),
        dir: "crates/kernel".to_owned(),
        role: members::Role::Product,
        reach: members::Reach::Workspace,
        depends_on: BTreeSet::new(),
    }]
}

/// A tool's files are not the product's, and this gate never walks to
/// them. The map still carries them for a reader, and carrying them may
/// not turn into judging them.
#[test]
fn an_entry_outside_the_product_is_carried_but_not_judged() {
    let mut violations = Vec::new();
    let found = rows(&map_of(&[REGISTERED, TOOL]), &product(), &mut violations);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, "crates/kernel/src/gate.rs");
    assert!(violations.is_empty());
}

/// Three ways one entry can be wrong, and each names the entry rather than
/// a line: a structured file is searched by name, and a name survives a
/// reordering that a line number does not.
#[test]
fn a_bad_status_a_duplicate_and_an_empty_duty_are_each_refused() {
    let mut violations = Vec::new();
    let found = rows(
        &map_of(&[BAD_STATUS, EMPTY_DUTY, REGISTERED, REGISTERED]),
        &product(),
        &mut violations,
    );
    assert_eq!(found.len(), 1);
    assert_eq!(violations.len(), 3);
    assert!(violations.iter().all(|held| held.location.contains(MAP)));
    assert!(
        violations
            .iter()
            .any(|held| held.violation.contains("empty `owns`"))
    );
}

#[test]
fn index_name_requires_registered_children() {
    let dirs: BTreeSet<String> = ["crates/runtime/src/tools".to_owned()].into();
    assert!(is_index_name("crates/runtime/src/tools.rs", &dirs));
    assert!(is_index_name("crates/kernel/src/lib.rs", &dirs));
    assert!(!is_index_name("crates/kernel/src/util.rs", &dirs));
}
