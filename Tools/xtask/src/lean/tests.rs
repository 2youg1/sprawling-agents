// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{arms, code, imports, inductives, module_file};

#[test]
fn comments_and_strings_are_blanked_and_the_lines_stay_where_they_were() {
    let text = "def a := 1 -- sorry\n/- outer /- inner -/ still -/ def b := \"sorry\\\"\"\nsorry";
    let blanked = code(text);
    assert_eq!(
        (
            blanked.lines().count(),
            blanked.contains("sorry"),
            blanked.lines().last()
        ),
        (3, true, Some("sorry"))
    );
    assert!(!blanked.lines().take(2).any(|line| line.contains("sorry")));
    assert!(blanked.contains("def b :="));
}

#[test]
fn a_roster_and_a_table_are_read_one_row_per_line() {
    let text = "inductive Reach where\n  | client\n  -- a comment row\n  | push (why : String)\n  \
                deriving Repr\n\ndef Reach.word : Reach → String\n  | .client => \"c\"\n  \
                | .push _ =>   .push   .x\ntheorem t : True := trivial\n";
    let blanked = code(text);
    let roster = inductives(&blanked);
    let table = arms(&blanked, "Reach.word").unwrap();
    let read: Vec<(usize, String, String)> = table
        .arms
        .into_iter()
        .map(|arm| (arm.line, arm.pattern, arm.value))
        .collect();
    assert_eq!(
        (
            roster["Reach"].line,
            roster["Reach"].names.clone(),
            table.line,
            read
        ),
        (
            1,
            vec!["client".to_owned(), "push".to_owned()],
            7,
            vec![
                (8, "client".to_owned(), String::new()),
                (9, "push".to_owned(), ".push .x".to_owned()),
            ]
        )
    );
    assert!(arms(&blanked, "Reach").is_none());
}

#[test]
fn imports_and_module_names_map_onto_files() {
    assert_eq!(
        (
            imports("import Lean.Data.Json\n-- import Sprawling\nimport crates.kernel.Spec Std\n"),
            module_file("crates.storage.spec.Jsonl.Barrier"),
        ),
        (
            vec![
                (1, "Lean.Data.Json".to_owned()),
                (3, "crates.kernel.Spec".to_owned()),
                (3, "Std".to_owned()),
            ],
            "crates/storage/spec/Jsonl/Barrier.lean".to_owned()
        )
    );
}
