// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::super::language::Line;
use super::super::language::{CONTROL, help, parse, snake, verbs};
use super::helpers::*;

/// The load-bearing property of this whole module: the verb table is
/// a projection of the wire's vocabulary. A hand-written list would
/// drift, and nothing would say so.
#[test]
fn every_wire_verb_is_a_verb_this_console_answers_to() {
    let known = verbs();
    for name in channels::COMMAND_NAMES.iter().chain(&channels::QUERY_NAMES) {
        assert!(
            known.contains(&snake(name)),
            "{name} is on the wire and not in the console"
        );
    }
}

/// A name that meant one thing on the wire and another in the
/// console would make `/cancel` ambiguous to a person and to this
/// parser at once.
#[test]
fn no_control_verb_shares_a_name_with_a_wire_verb() {
    for control in CONTROL {
        let clash = channels::COMMAND_NAMES
            .iter()
            .chain(&channels::QUERY_NAMES)
            .any(|name| snake(name) == control);
        assert!(!clash, "{control} means two things");
    }
}
#[test]
fn the_wire_spelling_becomes_the_typed_spelling() {
    assert_eq!(snake("AttachEndpoint"), "attach_endpoint");
    assert_eq!(snake("RunView"), "run_view");
    assert_eq!(snake("Fork"), "fork");
}
#[test]
fn an_empty_line_is_not_a_question() {
    assert_eq!(parse("   ", Some(&room())), Line::Nothing);
}
#[test]
fn plain_text_is_work_for_the_chosen_room() {
    assert_eq!(
        parse("  measure the beam  ", Some(&room())),
        Line::Work("measure the beam".to_owned())
    );
}

/// With nowhere for the work to go, the console says what to type
/// rather than guessing a room on somebody's behalf.
#[test]
fn plain_text_with_no_room_chosen_says_what_to_type() {
    let Line::Unknown { nearest, .. } = parse("measure the beam", None) else {
        panic!("work with no room is refused");
    };
    assert_eq!(nearest, vec!["at".to_owned()]);
}
#[test]
fn the_control_verbs_are_the_five_it_owns() {
    assert_eq!(parse("/help", None), Line::Help);
    assert_eq!(parse("/web", None), Line::OpenWeb);
    assert_eq!(parse("/serving", None), Line::Serving);
    assert_eq!(parse("/quit", None), Line::Quit);
    assert_eq!(parse("/at lab/room1", None), Line::Select(room()));
}
#[test]
fn a_query_with_no_arguments_is_the_bare_name() {
    let Line::Frame(frame) = parse("/city_view", None) else {
        panic!("city_view is a query");
    };
    assert!(matches!(
        *frame,
        channels::ClientFrame::Query(channels::Query::CityView)
    ));
}
#[test]
fn a_query_that_needs_an_argument_takes_it_as_json() {
    let Line::Frame(frame) = parse("/archive_search {\"needle\":\"beam\"}", None) else {
        panic!("archive_search takes a needle");
    };
    match *frame {
        channels::ClientFrame::Query(channels::Query::ArchiveSearch { needle }) => {
            assert_eq!(needle, "beam");
        }
        _ => panic!("the frame is the query that was named"),
    }
}
#[test]
fn an_unknown_verb_comes_back_with_the_ones_that_start_like_it() {
    let Line::Unknown { verb, nearest } = parse("/carn", None) else {
        panic!("carn is nobody's verb");
    };
    assert_eq!(verb, "carn");
    assert!(nearest.contains(&"cancel".to_owned()), "{nearest:?}");
}

/// A wire verb with a body it cannot read is a body problem, not a
/// verb problem, and the answer says so.
#[test]
fn a_known_verb_with_an_unreadable_body_is_told_apart_from_an_unknown_one() {
    let Line::Unknown { nearest, .. } = parse("/dispatch not json", None) else {
        panic!("dispatch needs a body it can read");
    };
    assert!(nearest[0].contains("JSON body"), "{nearest:?}");
}
#[test]
fn help_names_every_verb_the_parser_answers_to() {
    let text = help(Some(&room()));
    for name in channels::COMMAND_NAMES.iter().chain(&channels::QUERY_NAMES) {
        assert!(text.contains(&snake(name)), "{name} is missing from help");
    }
    assert!(text.contains("lab/room1"), "help says where work goes");
}
