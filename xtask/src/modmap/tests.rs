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

/// Built out of the workspace, so its file is not under `crates/`.
const OUTSIDE: &str = concat!(
    r#"{ name = "desktop::shell", file = "desktop/src/shell.rs", "#,
    r#"owns = "the window", shape = "adapter", since = "F1", "#,
    r#"status = "built", spec = "desktop-SPEC.md#8-1" }"#,
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

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level under the repository root")
        .to_path_buf()
}

/// `desktop` is built out of the workspace, so its files are not under
/// `crates/` and this gate never walks to them. The map still carries them
/// for a reader, and carrying them may not turn into judging them.
#[test]
fn an_entry_outside_crates_is_carried_but_not_judged() {
    let mut violations = Vec::new();
    let found = rows(&map_of(&[REGISTERED, OUTSIDE]), &mut violations);
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

/// The gate against the map it guards.
#[test]
fn this_repository_passes_its_own_module_map() {
    let violations = check(&root()).expect("the module map is readable");
    let said: Vec<String> = violations
        .iter()
        .map(|held| format!("{}: {}", held.location, held.violation))
        .collect();
    assert!(said.is_empty(), "{said:#?}");
}

#[test]
fn index_name_requires_registered_children() {
    let dirs: BTreeSet<String> = ["crates/runtime/src/tools".to_owned()].into();
    assert!(is_index_name("crates/runtime/src/tools.rs", &dirs));
    assert!(is_index_name("crates/kernel/src/lib.rs", &dirs));
    assert!(!is_index_name("crates/kernel/src/util.rs", &dirs));
}
