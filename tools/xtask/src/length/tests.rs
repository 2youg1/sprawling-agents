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
        .map(|found| (found.body.name, found.args))
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
    let root = crate::root::this_checkout().to_path_buf();
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
                still_over.insert(key(&rel, &item.body.name));
            }
        }
    }
    let spent: Vec<&String> = excused.difference(&still_over).collect();
    assert!(
        spent.is_empty(),
        "these are excused and no longer need to be; strike them:\n{spent:#?}"
    );
}

/// The gate reports a spent parameter exception in both shapes the
/// other two registers report: a name that has come back inside the
/// budget, and a name that is no longer in the tree, so a spent row is
/// something `just check` reports rather than something only a test
/// reads.
#[test]
fn a_spent_parameter_exception_is_reported_in_both_shapes() {
    let excused: BTreeSet<String> = ["a.rs::wide", "b.rs::narrowed", "c.rs::gone"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let still_here: BTreeMap<String, usize> = [("a.rs::wide", 7), ("b.rs::narrowed", 3)]
        .into_iter()
        .map(|(name, args)| (name.to_owned(), args))
        .collect();
    let found = spent_signatures(&excused, &still_here, 4);
    let said: Vec<&str> = found.iter().map(|one| one.violation.as_str()).collect();
    assert_eq!(said.len(), 2, "{found:#?}");
    assert!(said[0].contains("b.rs::narrowed takes 3"), "{said:#?}");
    assert!(said[1].contains("c.rs::gone is excused"), "{said:#?}");
    assert_eq!(
        found[0].rule, "an exception that is no longer needed is struck from the register",
        "the three registers state this rule in one wording"
    );
}

/// One register address can name more than one function: `workshop.rs`
/// holds an eight-parameter `new` and a one-parameter `new`. Reading
/// whichever was parsed last reported a live exception as spent, and
/// striking it would have let the wide one through.
#[test]
fn an_address_holding_two_functions_is_judged_by_the_wider_one() {
    let source = "\
        struct A;\n\
        struct B;\n\
        impl A { fn new(a: u8, b: u8, c: u8, d: u8, e: u8) -> Self { Self } }\n\
        impl B { fn new(a: u8) -> Self { Self } }\n";
    let parsed = syn::parse_file(source).unwrap();
    let mut widest: BTreeMap<String, usize> = BTreeMap::new();
    for found in measure(&parsed.items) {
        let held = widest
            .entry(key("x.rs", &found.body.name))
            .or_insert(found.args);
        *held = (*held).max(found.args);
    }
    assert_eq!(widest.get("x.rs::new"), Some(&5));
    let excused: BTreeSet<String> = ["x.rs::new".to_owned()].into_iter().collect();
    assert!(spent_signatures(&excused, &widest, 4).is_empty());
}

/// The register is the authority for both numbers, and both are read
/// from it by name. A row that stops stating its budget must fail
/// loudly rather than fall back to something this file believes.
#[test]
fn both_limits_come_from_the_register_and_neither_has_a_default() {
    let root = crate::root::this_checkout().to_path_buf();
    assert_eq!(limit(&root, ROW).unwrap(), 200);
    assert_eq!(limit(&root, FILE_ROW).unwrap(), 400);
    assert!(limit(&root, "a_row_nobody_wrote").is_err());
}

/// Every pin is a debt, so every pin must be above the budget it
/// excuses. A row at or below it is not an exception, it is a licence
/// somebody forgot to spend, and `no_longer_an_exception` reports it.
#[test]
fn every_pinned_file_is_over_the_budget_it_is_excused_from() {
    let root = crate::root::this_checkout().to_path_buf();
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

#[test]
fn the_file_budget_counts_production_lines_only() {
    let source = "fn production() {}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {}
}
";
    let parsed = syn::parse_file(source).unwrap();
    assert_eq!(production_lines(source, &parsed.items), 2);
}

fn lengths(source: &str) -> Vec<(String, usize)> {
    let parsed = syn::parse_file(source).unwrap();
    measure(&parsed.items)
        .into_iter()
        .map(|found| (found.body.name, found.body.lines))
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
fn methods_inside_an_impl_are_measured_one_by_one() {
    let found = lengths(
        "struct S;\nimpl S {\n    fn a(&self) {}\n    #[cfg(test)]\n    fn b(&self) {}\n}\n",
    );
    assert_eq!(found, vec![("a".to_owned(), 1)]);
}

/// The three budgets a fixture checkout states, at the numbers this
/// repository's register states.
const FIXTURE_REGISTER: &str = "[function_length]\nbudget_lines = 200\n\n\
    [argument_count]\nbudget_arguments = 4\n\n[file_length]\nbudget_lines = 400\n";

/// A Zig file is held to the file budget and its functions to the
/// function budget, both named at the file; a `test` declaration is
/// neither counted into the file nor measured, as `#[cfg(test)]` is not.
#[test]
fn a_long_zig_file_and_a_long_zig_function_are_named() {
    let root = crate::root::fixture::relocated("length-zig");
    crate::root::fixture::write(&root, REGISTER, FIXTURE_REGISTER);
    let mut zig = String::from("const std = @import(\"std\");\npub fn long() void {\n");
    zig.push_str(&"    _ = 0;\n".repeat(199));
    zig.push_str("}\n");
    for index in 0..200 {
        zig.push_str(&format!("const filler{index}: u8 = 0;\n"));
    }
    zig.push_str(
        "test \"a long test\" {\n    const helper = struct {\n        fn inner() void {\n",
    );
    zig.push_str(&"            _ = 0;\n".repeat(210));
    zig.push_str("        }\n    };\n    helper.inner();\n}\n");
    crate::root::fixture::write(&root, "tools/k/zig/long.zig", &zig);
    let found = check(&root).map(|all| {
        all.into_iter()
            .map(|v| format!("{}: {}", v.location, v.violation))
            .collect::<Vec<_>>()
    });
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        found.map_err(|err| err.to_string()),
        Ok(vec![
            "tools/k/zig/long.zig: 402 lines".to_owned(),
            "tools/k/zig/long.zig:2: long is 201 lines".to_owned(),
        ])
    );
}

#[test]
fn a_generated_page_is_exempt_but_unmarked_source_is_measured() {
    let root = crate::root::fixture::relocated("length-generated");
    crate::root::fixture::write(&root, REGISTER, FIXTURE_REGISTER);
    let source = "const line = 0;
".repeat(401);
    crate::root::fixture::write(&root, "client/src/plain.ts", &source);
    crate::root::fixture::write(
        &root,
        "client/src/assembled.ts",
        &format!("// Generated by `bun assemble.js`; edit the fragments.
{source}"),
    );
    let found = check(&root).map(|all| {
        all.into_iter()
            .map(|v| format!("{}: {}", v.location, v.violation))
            .collect::<Vec<_>>()
    });
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        found.map_err(|err| err.to_string()),
        Ok(vec!["client/src/plain.ts: 401 lines".to_owned()])
    );
}
