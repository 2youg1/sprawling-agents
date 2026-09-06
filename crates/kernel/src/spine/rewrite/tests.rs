// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::node_id::NodeId;
fn node(raw: &str) -> NodeId {
    NodeId::parse(raw).unwrap()
}
fn rows_of(text: &str) -> Vec<RoadmapRow> {
    match check_roadmap_shape(text) {
        RoadmapShape::WellFormed { rows } => rows,
        RoadmapShape::Malformed { problems } => panic!("expected well-formed: {problems:?}"),
    }
}

const GOOD: &str = "# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | chain verification | 1 |  | Done | cas:b3-abababababababababababababababababababababababababababababababab |
| 2 | range retrieval | 3 | 1 | In progress |  |
| 3 | fixtures | 1 |  | Awaiting approval |  |
| 4 | claimed without evidence | 1 |  | Done |  |
| 5 | evidence that does not parse | 1 |  | done | not-a-locator |
";
#[test]
fn a_rewritten_row_keeps_the_table_parsable_and_is_byte_stable() {
    let once = set_roadmap_status(GOOD, &node("2"), RoadmapStatus::InProgress, None).unwrap();
    let twice = set_roadmap_status(&once, &node("2"), RoadmapStatus::InProgress, None).unwrap();
    assert_eq!(once, twice, "the same edit twice is the same bytes");
    let rows = rows_of(&once);
    assert_eq!(rows.len(), 5, "editing one row does not lose the others");
    assert_eq!(rows[1].status, RoadmapStatus::InProgress);
    assert_eq!(rows[1].item, "range retrieval", "the item text survives");
    assert_eq!(rows[1].weight, 3, "and so do the weight and the needs");
    assert_eq!(rows[1].needs, vec![node("1")]);
}

#[test]
fn children_land_below_their_parent_and_number_on_from_the_last() {
    let text = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | build | 1 |  | Not started |  |
| 2 | ship | 1 |  | Not started |  |
";
    let split = insert_children(
        text,
        &node("1"),
        &[
            NewChild {
                item: "design".to_owned(),
                weight: 1,
            },
            NewChild {
                item: "code".to_owned(),
                weight: 3,
            },
        ],
    )
    .unwrap();
    let rows = rows_of(&split);
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["1", "1.1", "1.2", "2"], "reading order survives");
    assert_eq!(rows[2].weight, 3);

    let again = insert_children(
        &split,
        &node("1"),
        &[NewChild {
            item: "test".to_owned(),
            weight: 1,
        }],
    )
    .unwrap();
    let grown = rows_of(&again);
    let ids: Vec<&str> = grown.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(
        ids,
        ["1", "1.1", "1.2", "1.3", "2"],
        "a second split numbers on rather than reusing an index"
    );
}

#[test]
fn splitting_a_deep_branch_puts_the_children_after_the_whole_branch() {
    let text = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | build | 1 |  | Not started |  |
| 1.1 | design | 1 |  | Not started |  |
| 1.1.1 | sketch | 1 |  | Not started |  |
| 2 | ship | 1 |  | Not started |  |
";
    let split = insert_children(
        text,
        &node("1"),
        &[NewChild {
            item: "code".to_owned(),
            weight: 1,
        }],
    )
    .unwrap();
    let rows = rows_of(&split);
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["1", "1.1", "1.1.1", "1.2", "2"]);
}

#[test]
fn a_split_that_would_delete_or_hide_work_is_refused() {
    let text = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | build | 1 |  | Not started |  |
";
    assert!(insert_children(text, &node("1"), &[]).is_err());
    assert!(
        insert_children(
            text,
            &node("1"),
            &[NewChild {
                item: "  ".to_owned(),
                weight: 1
            }]
        )
        .is_err()
    );
    let zero = insert_children(
        text,
        &node("1"),
        &[NewChild {
            item: "x".to_owned(),
            weight: 0,
        }],
    )
    .unwrap_err();
    assert!(zero.recovery().contains("at least one"));
    let piped = insert_children(
        text,
        &node("1"),
        &[NewChild {
            item: "a | b".to_owned(),
            weight: 1,
        }],
    )
    .unwrap_err();
    assert!(piped.subject().contains("column separator"));
    assert!(
        insert_children(
            text,
            &node("9"),
            &[NewChild {
                item: "x".to_owned(),
                weight: 1
            }]
        )
        .is_err()
    );
}
