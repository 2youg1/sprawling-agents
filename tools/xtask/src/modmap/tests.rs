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

/// An entry outside the packages this gate judges is not walked to: the
/// map may carry it for a reader, and carrying it may not turn into
/// judging it.
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

/// A tool package other than the gates' own is judged as a product is:
/// a file its map does not register is a finding.
#[test]
fn a_tool_package_other_than_the_gates_is_judged() {
    let root = crate::root::fixture::relocated("modmap-tool");
    crate::root::fixture::write(
        &root,
        "tools/k/Cargo.toml",
        "[package]\nname = \"sprawling-k\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
         [lib]\nname = \"k\"\n\n[package.metadata.sprawling]\nrole = \"tool\"\n",
    );
    crate::root::fixture::write(&root, "tools/k/src/lib.rs", "mod a;\n");
    crate::root::fixture::write(&root, "tools/k/src/b.rs", "");
    let found = check(&root).map(|all| {
        all.into_iter()
            .map(|v| format!("{}: {}", v.location, v.violation))
            .collect::<Vec<_>>()
    });
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        found.map_err(|err| err.to_string()),
        Ok(vec![
            "tools/k/src/b.rs: file is not registered in the module map".to_owned()
        ])
    );
}
