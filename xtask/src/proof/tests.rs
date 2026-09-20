// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The roster reader, against the shapes this tree actually writes.

use super::roster::{in_file, module_base, strip};

/// The shape every harness in this tree has: an attribute, a function,
/// and an inline `mod verification` whose name is part of what kani
/// matches. Getting that segment wrong is invisible — `--harness` with
/// no match is a silent pass — so it is the first thing tested.
#[test]
fn a_harness_inside_an_inline_module_carries_that_module_in_its_name() {
    let text = "\
pub fn admit() {}

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn admit_is_total() {
        let capacity: u64 = kani::any();
    }
}
";
    let found = in_file(
        text,
        "kernel",
        "crates/kernel/src/backpressure.rs",
        "backpressure",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "backpressure::verification::admit_is_total");
    assert_eq!(found[0].package, "kernel");
    assert_eq!(found[0].location, "crates/kernel/src/backpressure.rs:7");
}

#[test]
fn two_harnesses_in_one_module_are_both_found() {
    let text = "\
#[cfg(kani)]
mod verification {
    #[kani::proof]
    fn first() {}

    #[kani::proof]
    fn second() {}
}
";
    let found = in_file(
        text,
        "kernel",
        "crates/kernel/src/discard/verdict.rs",
        "discard::verdict",
    );
    let names: Vec<&str> = found.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "discard::verdict::verification::first",
            "discard::verdict::verification::second"
        ]
    );
}

/// A function that merely sits below a harness must not inherit its
/// name: the attribute arms exactly one declaration.
#[test]
fn a_plain_function_after_a_harness_is_not_one() {
    let text = "\
mod verification {
    #[kani::proof]
    fn proved() {}

    fn helper() {}
}
";
    let found = in_file(text, "kernel", "crates/kernel/src/taint.rs", "taint");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "taint::verification::proved");
}

/// `mod x;` opens nothing. Counting it as a module would prefix every
/// later harness in the file with a module it never entered.
#[test]
fn a_module_declaration_without_a_body_opens_no_scope() {
    let text = "\
mod span;

#[cfg(kani)]
mod verification {
    #[kani::proof]
    fn total() {}
}
";
    let found = in_file(text, "kernel", "crates/kernel/src/secret.rs", "secret");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "secret::verification::total");
}

#[test]
fn a_brace_inside_a_string_or_a_comment_does_not_close_a_module() {
    let text = "\
mod verification {
    // }
    const SHAPE: &str = \"}\";

    #[kani::proof]
    fn still_inside() {}
}
";
    let found = in_file(
        text,
        "kernel",
        "crates/kernel/src/write_domain.rs",
        "write_domain",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "write_domain::verification::still_inside");
}

#[test]
fn a_file_is_its_own_module_except_the_crate_root() {
    assert_eq!(module_base("lib.rs"), Some(String::new()));
    assert_eq!(module_base("taint.rs"), Some("taint".to_owned()));
    assert_eq!(
        module_base("secret/scan.rs"),
        Some("secret::scan".to_owned())
    );
    assert_eq!(module_base("secret/mod.rs"), Some("secret".to_owned()));
    assert_eq!(module_base("main.rs"), None);
}

#[test]
fn a_harness_at_the_crate_root_has_no_leading_separator() {
    let text = "\
mod verification {
    #[kani::proof]
    fn rooted() {}
}
";
    let found = in_file(text, "kernel", "crates/kernel/src/lib.rs", "");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "verification::rooted");
}

#[test]
fn stripping_leaves_the_code_and_drops_the_prose() {
    let mut in_block = false;
    assert_eq!(strip("fn f() { // }", &mut in_block).trim(), "fn f() {");
    assert!(!in_block);
    assert_eq!(strip("let c = '}';", &mut in_block).trim(), "let c = ;");
    assert_eq!(strip("let s = \"{{\";", &mut in_block).trim(), "let s = ;");
}

#[test]
fn a_block_comment_spans_lines() {
    let mut in_block = false;
    assert_eq!(strip("/* {", &mut in_block).trim(), "");
    assert!(in_block);
    assert_eq!(strip("still } comment", &mut in_block).trim(), "");
    assert_eq!(strip("*/ fn f() {", &mut in_block).trim(), "fn f() {");
    assert!(!in_block);
}

/// The gate exists because these two homes existed. Both are read out of
/// the tree, so this test is the one that notices when the tree changes
/// shape rather than when the reader breaks.
#[test]
fn the_tree_holds_the_harnesses_the_workflow_no_longer_names() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let found = super::harnesses(&root).unwrap();
    assert!(
        found.len() >= 7,
        "the roster shrank to {}; if that was deliberate, ARCHITECTURE §11 states the new total",
        found.len()
    );
    assert!(
        found
            .iter()
            .any(|h| h.name == "backpressure::verification::admit_is_total_and_monotone_in_depth"),
        "the harness CI used to name by hand is no longer found by the reader that replaced it"
    );
    assert!(
        found
            .iter()
            .any(|h| h.name == "secret::scan::verification::log2_q10_is_total")
    );
}
