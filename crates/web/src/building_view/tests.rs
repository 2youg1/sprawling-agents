// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! Leaves and queues, through the production door.

use channels::{Address, BuildingAnswer, InboxAnswer};

use super::leaf::{Leaf, opening_leaf};
use super::room::{RoomQueue, waiting_in};

use super::leaf::room_addr;
use super::text::{class_of, pieces};
use channels::{ArchiveLine, BuildingDoc, PlannedProgress, Progress};

fn answer(docs: Vec<&str>) -> BuildingAnswer {
    BuildingAnswer {
        plan: Vec::new(),
        blocked: Vec::new(),
        sandbox: None,
        mcp: Vec::new(),
        addr: Address::parse("lab").unwrap(),
        progress: Progress::Planned(PlannedProgress {
            done: 3,
            blocked: 0,
            total: 7,
            done_ppb: 0,
            blocked_ppb: 0,
        }),
        problems: Vec::new(),
        rooms: vec!["room1".to_owned()],
        docs: docs
            .into_iter()
            .map(|name| BuildingDoc {
                name: name.to_owned(),
                text: "body".to_owned(),
                bytes: 4,
                truncated: false,
            })
            .collect(),
        archive: vec![ArchiveLine {
            kind: "decision".to_owned(),
            day: 20_000,
            subject: "we build without dx".to_owned(),
        }],
    }
}

#[test]
fn a_room_address_is_the_building_plus_the_directory_name() {
    let lab = Address::parse("lab").unwrap();
    assert_eq!(
        room_addr(&lab, "room1").map(|at| at.as_str().to_owned()),
        Some("lab/room1".to_owned())
    );
    // The city keeps the authority on what an address may hold; this
    // composes and lets the parser refuse.
    assert!(room_addr(&lab, "..").is_none());
    assert!(room_addr(&lab, "").is_none());
}

/// A building opens on the board when it has a plan, and on the
/// file when the plan does not parse: the board of an unreadable
/// plan is an empty board, which says nothing about what is wrong.
#[test]
fn a_building_opens_on_its_plan() {
    let mut held = answer(vec!["Roadmap.md", "Memo.md"]);
    assert_eq!(
        opening_leaf(&held),
        Leaf::Doc("Roadmap.md".to_owned()),
        "with no plan rows, the file is what there is to read"
    );
    held.plan = vec![channels::PlanRow {
        node: channels::NodeId::parse("1").unwrap(),
        item: "wire the kiln".to_owned(),
        status: channels::RoadmapStatus::NotStarted,
        share_ppb: channels::WHOLE_PPB,
        needs: Vec::new(),
        ready: true,
        leaf: true,
        evidence: None,
    }];
    assert_eq!(opening_leaf(&held), Leaf::Plan);
}

#[test]
fn a_building_with_no_documents_opens_on_what_it_has_filed() {
    assert_eq!(opening_leaf(&answer(Vec::new())), Leaf::Archive);
}

/// The whole document arrives, in order, whatever the lexer marked.
/// A view that dropped the bytes between two spans would silently
/// lose the prose that is most of any document an agent writes.
#[test]
fn splitting_a_document_loses_none_of_it_and_keeps_its_order() {
    let doc = "# 标题\n\n段落里有 `代码` 与 **重点**。\n\n- 一条\n\n```rust\nlet 值 = 1;\n```\n";
    let split = pieces(doc);
    let rejoined: String = split.iter().map(|(_, _, said)| said.as_str()).collect();
    assert_eq!(rejoined, doc, "a document is not a place to lose bytes");
    let offsets: Vec<usize> = split.iter().map(|(at, _, _)| *at).collect();
    let mut ascending = offsets.clone();
    ascending.sort_unstable();
    assert_eq!(offsets, ascending);
    assert!(
        split
            .iter()
            .any(|(_, piece, _)| *piece == Some(channels::Token::Heading)),
        "the heading is marked: {split:?}"
    );
}

/// A document with nothing to mark is still the document.
#[test]
fn plain_prose_arrives_whole_and_unmarked() {
    let split = pieces("just some words\n");
    assert_eq!(split.len(), 1);
    assert_eq!(split[0].1, None);
    assert_eq!(split[0].2, "just some words\n");
}

/// Colour cannot carry a token here: this design has two chromatic
/// tokens and both already mean something else, so every class has to
/// be separable by lightness and weight alone - and every one of them
/// has to actually be drawn.
#[test]
fn every_token_has_its_own_class_and_every_class_is_drawn() {
    let all = [
        channels::Token::Heading,
        channels::Token::Strong,
        channels::Token::Emphasis,
        channels::Token::Code,
        channels::Token::Fence,
        channels::Token::Meta,
        channels::Token::Link,
        channels::Token::Marker,
        channels::Token::Quote,
    ];
    let mut classes: Vec<&str> = all.iter().map(|token| class_of(*token)).collect();
    let held = classes.len();
    classes.sort_unstable();
    classes.dedup();
    assert_eq!(classes.len(), held, "two tokens cannot share one class");
    let sheet = include_str!("../../assets/app.css");
    for token in all {
        let class = class_of(token).replace(' ', ".");
        assert!(
            sheet.contains(&format!(".{class}")),
            "{class} is marked and never drawn"
        );
    }
}

fn queue_of(room: &str, ids: &[&str]) -> InboxAnswer {
    InboxAnswer {
        addr: Address::parse(&format!("lab/{room}")).unwrap(),
        waiting: ids
            .iter()
            .map(|id| channels::SignalLine {
                id: (*id).to_owned(),
                kind: "question".to_owned(),
                from: "lab/room2".to_owned(),
                at: channels::TimeMs::new(10),
            })
            .collect(),
    }
}

#[test]
fn another_rooms_answer_is_not_this_rooms_silence() {
    let lab = Address::parse("lab").unwrap();
    // The defect this refuses: showing "nothing waits here" on the
    // strength of an answer about a different room.
    assert_eq!(
        waiting_in(Some(&queue_of("room2", &[])), &lab, "room1"),
        RoomQueue::Unasked
    );
    assert_eq!(waiting_in(None, &lab, "room1"), RoomQueue::Unasked);
    assert_eq!(
        waiting_in(Some(&queue_of("room1", &[])), &lab, "room1"),
        RoomQueue::Empty
    );
    let waiting = waiting_in(Some(&queue_of("room1", &["sig-1", "sig-2"])), &lab, "room1");
    assert_eq!(
        waiting,
        RoomQueue::Waiting(queue_of("room1", &["sig-1", "sig-2"]).waiting)
    );
}

#[test]
fn progress_is_the_plans_own_numbers_or_an_admission() {
    // The words come from `web::progress`, which is the one place a
    // progress reading is written. This page used
    // to phrase its own, which is how the city page and this one came
    // to say the same thing two ways.
    let held = answer(vec!["Roadmap.md"]);
    let shown = crate::progress::bar(
        &held.progress,
        false,
        crate::progress::Subject::Plan,
        crate::lang::Lang::En,
    );
    assert_eq!(shown.label, "3/7");

    let mut unplanned = held;
    unplanned.progress = channels::Progress::Unplanned(channels::UnplannedProgress {
        steps: 0,
        budget: channels::BudgetUse::default(),
    });
    let said = crate::progress::bar(
        &unplanned.progress,
        false,
        crate::progress::Subject::Plan,
        crate::lang::Lang::En,
    );
    assert_eq!(said.label, "no plan");
    assert!(said.filled.is_none(), "a percentage nobody can compute");
}
