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

/// **The defect this card closes.** The console holds the same desk
/// the socket posts to, and until now it had no read path at all: a
/// question got an instruction to open a second terminal and ask the
/// city from outside, while the city was right here.
#[test]
fn a_question_is_answered_here_rather_than_sent_to_another_terminal() {
    let seen = typed("/metrics\n/quit\n", &terminal("127.0.0.1:8787", None));
    assert!(
        seen.contains("\"runs_active\":2"),
        "the answer itself is printed: {seen}"
    );
    assert!(
        !seen.contains("sprawling call"),
        "nobody is sent to a second terminal: {seen}"
    );
}

/// A refusal is the answer to some questions, and it reaches the
/// person who asked rather than a log file.
#[test]
fn a_question_this_city_refuses_comes_back_with_its_recovery() {
    let seen = typed("/cost_view\n/quit\n", &terminal("127.0.0.1:8787", None));
    assert!(seen.contains("CostView is not scripted here"), "{seen}");
}

/// The readout the person asked for: which port, how much is
/// running, and the one identifier that makes the memory
/// measurement a paste rather than a hunt.
#[test]
fn the_serving_screen_names_the_port_what_runs_and_this_process() {
    let screen = serving(&terminal("127.0.0.1:8787", None), Some(&vitals()), 24_188);
    assert!(screen.contains("127.0.0.1:8787"), "{screen}");
    assert!(screen.contains("this machine only"), "{screen}");
    assert!(screen.contains("2 active, 41 frozen"), "{screen}");
    assert!(screen.contains("3 approval(s)"), "{screen}");
    assert!(screen.contains("pid 24188"), "{screen}");
    assert!(screen.contains("cargo xtask mem 24188"), "{screen}");
}

/// Reach and key are two facts, and the dangerous cell is the one
/// worth shouting about: reachable from the network, nothing asked
/// of whoever arrives.
#[test]
fn the_door_line_tells_the_four_cells_apart() {
    let door = |bind: &str, token: Option<&str>| {
        serving(&terminal(bind, token), Some(&vitals()), 1)
            .lines()
            .find_map(|line| line.trim().strip_prefix("door      ").map(str::to_owned))
            .expect("every screen states its door")
    };
    assert!(door("127.0.0.1:8787", None).starts_with("none"));
    assert_eq!(
        door("127.0.0.1:8787", Some("k")),
        "a pairing key is required"
    );
    assert_eq!(door("0.0.0.0:8787", Some("k")), "a pairing key is required");
    assert!(
        door("0.0.0.0:8787", None).starts_with("NONE"),
        "an unkeyed listener beyond loopback is the cell to shout about"
    );
}

/// A city too busy to answer still has a port, and that half is the
/// half somebody locked out of the WebUI came for.
#[test]
fn the_screen_keeps_the_listener_half_when_the_city_does_not_answer() {
    let screen = serving(&terminal("127.0.0.1:8787", None), None, 1);
    assert!(screen.contains("127.0.0.1:8787"), "{screen}");
    assert!(screen.contains("views may be rebuilding"), "{screen}");
}
