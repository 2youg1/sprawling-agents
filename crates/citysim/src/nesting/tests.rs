// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Attempt, Fault, Grades, Shape, Verdict, grade, recommended, tally};
use std::collections::BTreeMap;

fn wanting(path: &str, value: &str) -> BTreeMap<String, String> {
    let mut wanted = BTreeMap::new();
    wanted.insert(path.to_owned(), value.to_owned());
    wanted
}

fn attempt(shape: Shape, before: &str, returned: &str) -> Attempt {
    Attempt {
        shape,
        before: before.to_owned(),
        returned: returned.to_owned(),
        touching: vec!["plan/one/state".to_owned()],
        wanted: wanting("plan/one/state", "done"),
    }
}

const JSON_BEFORE: &str =
    r#"{"plan":{"one":{"state":"open","weight":"3"},"two":{"state":"open"}}}"#;

#[test]
fn a_clean_edit_has_no_fault() {
    let after = r#"{"plan":{"one":{"state":"done","weight":"3"},"two":{"state":"open"}}}"#;
    let verdict = grade(&attempt(Shape::Json, JSON_BEFORE, after)).unwrap();
    assert_eq!(verdict.fault, None);
}

/// The finding this whole suite exists to make visible: a result
/// that parses and is missing a node is the worst outcome, because
/// it is the only one nothing downstream notices.
#[test]
fn a_document_that_parses_with_a_node_missing_is_the_worst_outcome() {
    let after = r#"{"plan":{"one":{"state":"done","weight":"3"}}}"#;
    let verdict = grade(&attempt(Shape::Json, JSON_BEFORE, after)).unwrap();
    assert_eq!(verdict.fault, Some(Fault::LostField));
    assert_eq!(Fault::ALL[0], Fault::LostField, "and it is ordered first");
}

/// Losing a field outranks failing to apply the edit: a result can
/// be several kinds of wrong, and reporting the mildest would
/// flatter the format.
#[test]
fn the_worst_fault_is_the_one_reported() {
    let after = r#"{"plan":{"one":{"state":"open","weight":"3"}}}"#;
    let verdict = grade(&attempt(Shape::Json, JSON_BEFORE, after)).unwrap();
    assert_eq!(verdict.fault, Some(Fault::LostField));
}

#[test]
fn a_value_nobody_asked_about_may_not_move() {
    let after = r#"{"plan":{"one":{"state":"done","weight":"5"},"two":{"state":"open"}}}"#;
    let verdict = grade(&attempt(Shape::Json, JSON_BEFORE, after)).unwrap();
    assert_eq!(verdict.fault, Some(Fault::ChangedBystander));
}

#[test]
fn an_edit_that_never_landed_is_counted() {
    let after = r#"{"plan":{"one":{"state":"open","weight":"3"},"two":{"state":"open"}}}"#;
    let verdict = grade(&attempt(Shape::Json, JSON_BEFORE, after)).unwrap();
    assert_eq!(verdict.fault, Some(Fault::NotApplied));
}

/// Running out of room and writing nonsense call for different
/// answers, and only one of them is fixed by asking for more tokens.
#[test]
fn stopping_early_is_told_apart_from_writing_nonsense() {
    let cut = r#"{"plan":{"one":{"state":"done","#;
    assert_eq!(
        grade(&attempt(Shape::Json, JSON_BEFORE, cut))
            .unwrap()
            .fault,
        Some(Fault::Truncated)
    );
    let wrong = r#"{"plan": <<<}"#;
    assert_eq!(
        grade(&attempt(Shape::Json, JSON_BEFORE, wrong))
            .unwrap()
            .fault,
        Some(Fault::Unparseable)
    );
}

#[test]
fn all_three_shapes_are_read_into_the_same_leaves() {
    let toml = "[plan.one]\nstate = \"open\"\nweight = \"3\"\n[plan.two]\nstate = \"open\"\n";
    let after = "[plan.one]\nstate = \"done\"\nweight = \"3\"\n[plan.two]\nstate = \"open\"\n";
    assert_eq!(
        grade(&attempt(Shape::Toml, toml, after)).unwrap().fault,
        None
    );

    let md = "- plan\n  - one\n    - state: open\n    - weight: 3\n  - two\n    - state: open\n";
    let md_after =
        "- plan\n  - one\n    - state: done\n    - weight: 3\n  - two\n    - state: open\n";
    assert_eq!(
        grade(&attempt(Shape::Markdown, md, md_after))
            .unwrap()
            .fault,
        None
    );
}

/// A reader that repaired sloppy indentation would hide the exact
/// failure this suite counts.
#[test]
fn a_nested_list_with_a_broken_indent_does_not_quietly_parse() {
    let md = "- plan\n  - one\n    - state: open\n";
    let bent = "- plan\n   - one\n    - state: done\n";
    assert_eq!(
        grade(&Attempt {
            shape: Shape::Markdown,
            before: md.to_owned(),
            returned: bent.to_owned(),
            touching: vec!["plan/one/state".to_owned()],
            wanted: wanting("plan/one/state", "done"),
        })
        .unwrap()
        .fault,
        Some(Fault::Unparseable)
    );
}

/// A broken fixture is not a model failure. Counting it as one would
/// let the corpus grade itself.
#[test]
fn a_corpus_that_does_not_parse_refuses_rather_than_scoring() {
    let broken = grade(&attempt(Shape::Json, "{not json", "{}"));
    assert!(broken.is_err());
}

#[test]
fn a_shape_nobody_tried_is_reported_rather_than_left_out() {
    let grades = tally(&[Verdict {
        shape: Shape::Json,
        fault: None,
    }]);
    assert_eq!(grades.len(), 3);
    assert_eq!(grades[1].shape, Shape::Json);
    assert_eq!(grades[1].tried, 1);
    let untried: Vec<Shape> = grades
        .iter()
        .filter(|held| held.tried == 0)
        .map(|held| held.shape)
        .collect();
    assert_eq!(untried, vec![Shape::Toml, Shape::Markdown]);
}

#[test]
fn the_rate_is_an_exact_integer() {
    let grades = tally(&[
        Verdict {
            shape: Shape::Toml,
            fault: Some(Fault::LostField),
        },
        Verdict {
            shape: Shape::Toml,
            fault: None,
        },
        Verdict {
            shape: Shape::Toml,
            fault: None,
        },
    ]);
    assert_eq!(grades[0].tried, 3);
    assert_eq!(grades[0].wrong, 1);
    assert_eq!(grades[0].wrong_per_mille(), 333);
}

/// Two formats that fail equally often are not equally good: the one
/// whose failures leave a readable file with a node missing costs
/// more, because only one of them is noticed.
#[test]
fn a_tie_is_broken_by_how_badly_each_one_fails() {
    let mut lost = BTreeMap::new();
    lost.insert(Fault::LostField, 1);
    let mut unreadable = BTreeMap::new();
    unreadable.insert(Fault::Unparseable, 1);
    let grades = vec![
        Grades {
            shape: Shape::Toml,
            tried: 10,
            wrong: 1,
            faults: lost,
        },
        Grades {
            shape: Shape::Json,
            tried: 10,
            wrong: 1,
            faults: unreadable,
        },
    ];
    assert_eq!(recommended(&grades), Some(Shape::Json));
}

#[test]
fn nothing_tried_recommends_nothing() {
    assert_eq!(recommended(&tally(&[])), None);
}
