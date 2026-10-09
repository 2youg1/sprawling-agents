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
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]

use super::super::language::Line;
use super::super::language::{carried_commands, help, parse, snake, wire_verbs};
use super::helpers::*;

/// The load-bearing property of `/wire`: its verbs are a projection
/// of the wire's vocabulary. A hand-written list would drift, and
/// nothing would say so.
#[test]
fn every_wire_verb_is_a_verb_this_console_answers_to() {
    let known = wire_verbs();
    for name in carried_commands().chain(wire::QUERY_NAMES.iter().copied()) {
        assert!(
            known.contains(&snake(name)),
            "{name} is on the wire and not in the console"
        );
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
    assert_eq!(parse("   ", key()), Line::Nothing);
}
#[test]
fn plain_text_is_work_for_the_chosen_room() {
    assert_eq!(
        parse("  measure the beam  ", key()),
        Line::Work("measure the beam".to_owned())
    );
}
#[test]
fn the_slash_verbs_are_the_tables() {
    assert_eq!(parse("/help", key()), Line::Help);
    assert_eq!(parse("/web", key()), Line::OpenWeb);
    assert_eq!(parse("/serving", key()), Line::Serving);
    assert_eq!(parse("/quit", key()), Line::Quit);
    assert_eq!(parse("/room lab/room1", key()), Line::Select(room()));
    assert_eq!(
        parse("/remote close", key()),
        Line::Remote(crate::outside::console::RemoteLine::Close)
    );
}
#[test]
fn a_query_with_no_arguments_is_the_bare_name() {
    let Line::Frame(frame) = parse("/wire city_view", key()) else {
        panic!("city_view is a query");
    };
    assert!(matches!(
        *frame,
        wire::ClientFrame::Ask(wire::Ask {
            query: wire::Query::CityView,
            ..
        })
    ));
}
#[test]
fn a_query_that_needs_an_argument_takes_it_as_json() {
    let Line::Frame(frame) = parse("/wire archive_search {\"needle\":\"beam\"}", key()) else {
        panic!("archive_search takes a needle");
    };
    match *frame {
        wire::ClientFrame::Ask(wire::Ask {
            query: wire::Query::ArchiveSearch { needle },
            ..
        }) => {
            assert_eq!(needle, "beam");
        }
        _ => panic!("the frame is the query that was named"),
    }
}
#[test]
fn an_unknown_verb_comes_back_with_the_ones_that_start_like_it() {
    let Line::Unknown { verb, nearest } = parse("/wire carn", key()) else {
        panic!("carn is nobody's verb");
    };
    assert_eq!(verb, "wire carn");
    assert!(nearest.contains(&"cancel".to_owned()), "{nearest:?}");
}

/// A wire verb with a body it cannot read is a body problem, not a
/// verb problem, and the answer says so.
#[test]
fn a_known_verb_with_an_unreadable_body_is_told_apart_from_an_unknown_one() {
    let Line::Malformed { verb, .. } = parse("/wire dispatch not json", key()) else {
        panic!("dispatch needs a body it can read");
    };
    assert_eq!(verb, "dispatch");
}
#[test]
fn help_names_every_verb_the_parser_answers_to() {
    let text = help(&room());
    for name in carried_commands().chain(wire::QUERY_NAMES.iter().copied()) {
        assert!(text.contains(&snake(name)), "{name} is missing from help");
    }
    assert!(text.contains("lab/room1"), "help says where work goes");
    assert!(text.contains("/room"), "help lists the slash verbs");
}
