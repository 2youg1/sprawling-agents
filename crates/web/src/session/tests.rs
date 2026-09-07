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

//! The head through the production door.

use super::facts::head_facts;
use super::links::building_of;
use super::tabs::Tab;
use crate::app::RunRow;
use crate::lang::Lang;
use crate::phase::Phase;

fn row() -> RunRow {
    RunRow {
        addr: channels::Address::parse("lab/parser").ok(),
        session: Some("parser".to_owned()),
        parent: None,
        phase: Phase::Running,
        steps_done: 0,
        steps_planned: None,
        started_at_seq: channels::Seq::FIRST,
        last_seq: channels::Seq::FIRST,
        turns: 7,
        handoff_at_turn: None,
        gate: None,
        spent: None,
        said: None,
        task: Some("read the ledger path twice".to_owned()),
    }
}

/// The head says four things and only four. A page that reported
/// whatever it happened to know would report a different set each
/// time it was opened.
///
/// The tabs are a separate count on purpose: the head answers what a
/// person arrives with, and a tab is a place to go afterwards. The
/// fifth is what the session was sent, which is a reading of the
/// same work rather than a fifth question at the door.
#[test]
fn the_head_answers_four_questions_and_no_others() {
    assert_eq!(head_facts(Lang::En, &row()).len(), 4);
    assert_eq!(Tab::ALL.len(), 5);
}

/// The whole of the fourth question's answer: the city is not asked
/// to guess. This is the assertion the card closes on.
#[test]
fn the_context_left_is_a_rule_and_never_a_number() {
    let mut row = row();
    row.turns = 40;
    row.spent = Some(channels::UsdMicros::new(1_360_000));
    let facts = head_facts(Lang::En, &row);
    let context = &facts[2];
    assert!(!context.known, "the wire does not carry this one");
    assert!(
        context.said.contains("——"),
        "an unanswerable fact is drawn as a rule, not as a figure: {}",
        context.said
    );
    assert!(
        !context.said.chars().any(|glyph| glyph.is_ascii_digit()),
        "a number here would be invented: {}",
        context.said
    );
}

/// A session nobody has priced does not read as a free one.
#[test]
fn an_unpriced_session_says_so_rather_than_saying_zero() {
    let facts = head_facts(Lang::En, &row());
    assert!(!facts[0].known);
    assert!(!facts[0].said.contains("0.00"));
}

/// The rule the streaming half closes on: an increment is something
/// to watch, and the record is what is true.
///
/// A provider that revises, or a stream cut halfway, leaves text in
/// the buffer that no `model_returned` ever confirmed. Dropping the
/// buffer when the record lands is the whole of the rule, and it is
/// asserted on the fold rather than on the markup because the fold is
/// where it is decided.
#[test]
fn where_the_increments_and_the_record_disagree_the_record_wins() {
    // Seated first, because a run this client never saw start has no
    // row to carry the settled text - which is a different defect
    // from the one under test.
    let mut snapshot = crate::app::seated(&[(Some("lab/parser"), Phase::Running, 1)]);
    let run = channels::RunId::from_bytes([0u8; 16]);
    snapshot.is_saying(&channels::Delta {
        run,
        text: "half a sen".to_owned(),
    });
    assert_eq!(snapshot.saying(&run), Some("half a sen"));
    snapshot.is_saying(&channels::Delta {
        run,
        text: "tence".to_owned(),
    });
    assert_eq!(
        snapshot.saying(&run),
        Some("half a sentence"),
        "increments join; keeping only the last would show every fortieth word"
    );

    snapshot.apply(&crate::app::returned_for_test(run, "the whole sentence", 9));
    assert_eq!(
        snapshot.saying(&run),
        None,
        "the call settled, so nothing is still being said"
    );
    let Some(row) = snapshot.run(&run) else {
        panic!("the record seats the run");
    };
    assert_eq!(
        row.said.as_deref(),
        Some("the whole sentence"),
        "the page draws what the record says, not what arrived before it"
    );
}

/// The three answerable ones are answered from records this client
/// already folds, which is what makes them free.
#[test]
fn the_three_answerable_facts_come_from_the_stream() {
    let mut row = row();
    row.spent = Some(channels::UsdMicros::new(420_000));
    row.gate = Some("exec".to_owned());
    row.handoff_at_turn = Some(4);
    let facts = head_facts(Lang::En, &row);
    assert!(facts[0].known && facts[0].said.contains("$0.42"));
    assert!(facts[1].known && facts[1].said.contains("exec"));
    assert!(
        facts[3].known && facts[3].said.contains('3'),
        "7 - 4 turns ago"
    );
}

/// A handoff written this turn is current, and saying "0 turns ago"
/// makes a reader work out that it means now.
#[test]
fn a_handoff_written_this_turn_says_this_turn() {
    let mut row = row();
    row.handoff_at_turn = Some(7);
    let facts = head_facts(Lang::En, &row);
    assert_eq!(facts[3].said, "handoff written this turn");
}

#[test]
fn a_room_knows_which_building_it_is_in() {
    let addr = channels::Address::parse("lab/parser").unwrap();
    assert_eq!(building_of(&addr).unwrap().as_str(), "lab");
    let bare = channels::Address::parse("lab").unwrap();
    assert_eq!(building_of(&bare), None);
}
