// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// A receiver is not a parameter, and a data clump is.
#[test]
fn a_receiver_is_not_an_argument_and_everything_else_is() {
    let source = "\
        struct S;\n\
        impl S {\n\
          fn free() {}\n\
          fn borrowed(&self) {}\n\
          fn owned(self, a: u8) {}\n\
          fn mutable(&mut self, a: u8, b: u8, c: u8, d: u8) {}\n\
          fn clump(&self, a: u8, b: u8, c: u8, d: u8, e: u8) {}\n\
        }\n";
    let parsed = syn::parse_file(source).unwrap();
    let counted: Vec<(String, usize)> = measure(&parsed.items)
        .into_iter()
        .map(|found| (found.name, found.args))
        .collect();
    assert_eq!(
        counted,
        [
            ("free".to_owned(), 0),
            ("borrowed".to_owned(), 0),
            ("owned".to_owned(), 1),
            ("mutable".to_owned(), 4),
            ("clump".to_owned(), 5),
        ]
    );
}

/// Every excused signature must still exist and must still be over
/// the budget. A name that no longer names anything is a licence
/// waiting to be spent by whoever writes that function next.
#[test]
fn every_excused_signature_is_a_real_one_that_is_still_over() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .expect("xtask lives one level under the repo root");
    let budget = limit(&root, ARG_ROW).unwrap();
    let excused = excused(&root).unwrap();
    assert!(!excused.is_empty(), "the register records the debt it owes");
    let shapes = modmap::shapes(&root).unwrap();
    let mut still_over = BTreeSet::new();
    for file in sources(&root).unwrap() {
        let rel = walk::rel(&root, &file);
        if shapes.get(&rel).is_some_and(|shape| shape == "data") {
            continue;
        }
        let text = walk::read_text(&file).unwrap();
        let parsed = syn::parse_file(&text).unwrap();
        for item in measure(&parsed.items) {
            if item.args > budget {
                still_over.insert(key(&rel, &item.name));
            }
        }
    }
    let spent: Vec<&String> = excused.difference(&still_over).collect();
    assert!(
        spent.is_empty(),
        "these are excused and no longer need to be; strike them:\n{spent:#?}"
    );
}

/// The register is the authority for both numbers, and both are read
/// from it by name. A row that stops stating its budget must fail
/// loudly rather than fall back to something this file believes.
#[test]
fn both_limits_come_from_the_register_and_neither_has_a_default() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .expect("xtask lives one level under the repo root");
    assert_eq!(limit(&root, ROW).unwrap(), 200);
    assert_eq!(limit(&root, FILE_ROW).unwrap(), 400);
    assert!(limit(&root, "a_row_nobody_wrote").is_err());
}

/// Every pin is a debt, so every pin must be above the budget it
/// excuses. A row at or below it is not an exception, it is a licence
/// somebody forgot to spend, and `no_longer_an_exception` reports it.
#[test]
fn every_pinned_file_is_over_the_budget_it_is_excused_from() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .expect("xtask lives one level under the repo root");
    let budget = limit(&root, FILE_ROW).unwrap();
    let pinned = predating(&root).unwrap();
    // No assertion that the register is non-empty. It held while
    // there was a file left to split and turned red the moment the
    // last one landed, which made finishing the work look like
    // breaking the gate. What is worth holding is the property each
    // row must have, over however many rows there are - and an empty
    // register is this rule's finished state, not its failure.
    for (rel, lines) in &pinned {
        assert!(
            *lines > budget,
            "{rel} is pinned at {lines}, which the {budget}-line rule already allows"
        );
        assert!(
            root.join(rel).exists(),
            "{rel} is pinned and not in the tree"
        );
    }
}

fn lengths(source: &str) -> Vec<(String, usize)> {
    let parsed = syn::parse_file(source).unwrap();
    measure(&parsed.items)
        .into_iter()
        .map(|found| (found.name, found.lines))
        .collect()
}

#[test]
fn a_brace_inside_a_literal_is_not_a_block() {
    // The counting bug that turned a 3-line function into the rest
    // of its file: `'{'` is a character, and `"{"` is a string.
    let found = lengths(
        "fn detect(t: &str) -> bool {\n    t.starts_with('{') || t.ends_with(\"}\")\n}\nfn after() {}\n",
    );
    assert_eq!(
        found,
        vec![("detect".to_owned(), 3), ("after".to_owned(), 1)]
    );
}

#[test]
fn cfg_test_marks_an_item_and_not_the_rest_of_the_file() {
    let found = lengths(
        "#[cfg(test)]\nfn helper() {\n    let _ = 1;\n}\nfn production() {\n    let _ = 2;\n}\n",
    );
    assert_eq!(found, vec![("production".to_owned(), 3)]);
}

#[test]
fn a_component_is_markup_and_a_plain_function_is_not() {
    let found = lengths(
        "#[component]\nfn Page() -> Element {\n    rsx! { div {} }\n}\nfn plain() -> u8 {\n    1\n}\n",
    );
    assert_eq!(found, vec![("plain".to_owned(), 3)]);
}

#[test]
fn methods_inside_an_impl_are_measured_one_by_one() {
    let found = lengths(
        "struct S;\nimpl S {\n    fn a(&self) {}\n    #[cfg(test)]\n    fn b(&self) {}\n}\n",
    );
    assert_eq!(found, vec![("a".to_owned(), 1)]);
}
