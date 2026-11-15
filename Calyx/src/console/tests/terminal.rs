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

use super::super::terminal::serving;
use super::helpers::*;

#[test]
fn a_question_is_answered_here_rather_than_sent_to_another_terminal() {
    let seen = typed("/wire metrics\n", &terminal("127.0.0.1:2501", None));
    assert!(
        seen.contains("\"runs_active\":2"),
        "the answer itself is printed: {seen}"
    );
    assert!(
        !seen.contains("sprawling call"),
        "nobody is sent to a second terminal: {seen}"
    );
}

#[test]
fn a_question_this_city_refuses_comes_back_with_its_recovery() {
    let seen = typed("/wire cost_view\n", &terminal("127.0.0.1:2501", None));
    assert!(seen.contains("CostView is not scripted here"), "{seen}");
}

#[test]
fn the_serving_screen_names_the_port_what_runs_and_this_process() {
    let screen = serving(&terminal("127.0.0.1:2501", None), Some(&vitals()), 24_188);
    assert!(screen.contains("127.0.0.1:2501"), "{screen}");
    assert!(screen.contains("this machine only"), "{screen}");
    assert!(screen.contains("2 active, 41 frozen"), "{screen}");
    assert!(screen.contains("3 approval(s)"), "{screen}");
    assert!(screen.contains("pid 24188"), "{screen}");
    assert!(screen.contains("cargo xtask mem 24188"), "{screen}");
}

#[test]
fn the_door_line_tells_the_four_cells_apart() {
    let door = |bind: &str, token: Option<&str>| {
        serving(&terminal(bind, token), Some(&vitals()), 1)
            .lines()
            .find_map(|line| line.trim().strip_prefix("door      ").map(str::to_owned))
            .expect("every screen states its door")
    };
    assert!(door("127.0.0.1:2501", None).contains("key file"));
    assert_eq!(
        door("127.0.0.1:2501", Some("k")),
        "a pairing key is required"
    );
    assert_eq!(door("0.0.0.0:2501", Some("k")), "a pairing key is required");
    assert!(door("0.0.0.0:2501", None).contains("you configured"));
}

#[test]
fn the_screen_keeps_the_listener_half_when_the_city_does_not_answer() {
    let screen = serving(&terminal("127.0.0.1:2501", None), None, 1);
    assert!(screen.contains("127.0.0.1:2501"), "{screen}");
    assert!(screen.contains("views may be rebuilding"), "{screen}");
}

fn posted(script: &str) -> Vec<wire::Command> {
    let inside = inside();
    let desk = &inside.desk;
    driven(script, &terminal("127.0.0.1:2501", None), &inside);
    std::iter::from_fn(|| match desk.next(|_run| false) {
        accounting::worker::DeskWait::Command(posted, _underway) => Some(posted.command),
        accounting::worker::DeskWait::Idle
        | accounting::worker::DeskWait::Close(_)
        | accounting::worker::DeskWait::Gone => None,
    })
    .collect()
}

#[test]
fn the_same_line_typed_twice_is_two_dispatches() {
    let keys: Vec<kernel::IdemKey> = posted("/room lab/room1\nhello there\nhello there\n")
        .iter()
        .filter_map(|command| command.idem().copied())
        .collect();
    assert_eq!(keys.len(), 2, "both lines reach the desk: {keys:?}");
    assert_ne!(keys[0], keys[1], "each line carries its own key");
}

#[test]
fn a_wire_verb_without_a_key_gets_the_line_key() {
    let commands = posted("/wire halt {\"scope\":\"city\"}\n");
    assert!(
        matches!(commands.as_slice(), [wire::Command::Halt { .. }]),
        "the halt reaches the desk: {} command(s)",
        commands.len()
    );
}

#[test]
fn a_body_missing_a_field_names_the_field() {
    let seen = typed("/wire halt {}\n", &terminal("127.0.0.1:2501", None));
    assert!(
        seen.contains("`scope`"),
        "the missing field is named: {seen}"
    );
    assert!(!seen.contains("no verb"), "halt is a verb: {seen}");
}

#[test]
fn help_offers_only_what_the_console_can_carry() {
    let seen = typed(
        "/help\n/wire auth {\"token\":\"x\"}\n",
        &terminal("127.0.0.1:2501", None),
    );
    assert!(!seen.contains("put_secret"), "{seen}");
    assert!(!seen.contains(", auth"), "{seen}");
    assert!(
        seen.contains("no verb `/wire auth"),
        "auth is refused as a verb: {seen}"
    );
}
