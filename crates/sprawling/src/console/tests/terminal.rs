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

use super::super::Records;
use super::super::terminal::{printed, serving};
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

/// Runs the console loop and returns every Command it posted.
fn posted(script: &str) -> Vec<wire::Command> {
    let inside = inside();
    let desk = &inside.desk;
    let mut out: Vec<u8> = Vec::new();
    super::super::terminal::drive(
        &terminal("127.0.0.1:8787", None),
        &inside,
        &mut std::io::Cursor::new(script.as_bytes().to_vec()),
        &mut out,
    );
    // Read through the door the writer's loop reads through, so the test
    // sees exactly what the loop would be handed.
    std::iter::from_fn(|| match desk.next(|_run| false) {
        accounting::worker::DeskWait::Command(posted, _underway) => Some(posted.command),
        accounting::worker::DeskWait::Idle
        | accounting::worker::DeskWait::Close(_)
        | accounting::worker::DeskWait::Gone => None,
    })
    .collect()
}

/// The city answers a key it has seen with its first answer, across
/// restarts. A line typed twice is two requests, and a line typed again
/// after its refusal was fixed must reach the city a second time.
#[test]
fn the_same_line_typed_twice_is_two_dispatches() {
    let keys: Vec<kernel::IdemKey> = posted("/at lab/room1\nhello there\nhello there\n")
        .iter()
        .filter_map(|command| command.idem().copied())
        .collect();
    assert_eq!(keys.len(), 2, "both lines reach the desk: {keys:?}");
    assert_ne!(keys[0], keys[1], "each line carries its own key");
}

/// Braking the whole city from a machine with no browser is one short
/// line: the console supplies the key the wire requires.
#[test]
fn a_wire_verb_without_a_key_gets_the_line_key() {
    let commands = posted("/halt {\"scope\":\"city\"}\n");
    assert!(
        matches!(commands.as_slice(), [wire::Command::Halt { .. }]),
        "the halt reaches the desk: {} command(s)",
        commands.len()
    );
}

/// A known verb whose body cannot be read says which field is wrong,
/// not that the verb does not exist.
#[test]
fn a_body_missing_a_field_names_the_field() {
    let seen = typed("/halt {}\n", &terminal("127.0.0.1:8787", None));
    assert!(
        seen.contains("`scope`"),
        "the missing field is named: {seen}"
    );
    assert!(!seen.contains("no verb"), "halt is a verb: {seen}");
}

/// The help lists what a socket can carry and says what `/quit` does.
#[test]
fn help_offers_only_what_the_console_can_carry() {
    let seen = typed(
        "/help\n/auth {\"token\":\"x\"}\n",
        &terminal("127.0.0.1:8787", None),
    );
    assert!(!seen.contains("put_secret"), "{seen}");
    assert!(!seen.contains(", auth"), "{seen}");
    assert!(
        seen.contains("no verb `auth"),
        "auth is refused as a verb: {seen}"
    );
    assert!(seen.contains("the city keeps serving"), "{seen}");
}

/// A committed dispatch, carrying words the User typed.
fn dispatched() -> kernel::EventRecord {
    let draft = kernel::EventDraft {
        run: kernel::RunId::CITY,
        t: kernel::TimeMs::new(1_778_749_200_000),
        who: "tester".to_owned(),
        addr: Some(room()),
        kind: kernel::EventKind::SteerReceived,
        data: kernel::Payload::new(
            serde_json::json!({"text": "the secret plan"})
                .as_object()
                .cloned()
                .unwrap(),
        )
        .unwrap(),
        ig: false,
    };
    kernel::EventRecord::from_draft(draft, kernel::Seq::new(42), kernel::GENESIS_PREV)
}

/// A14: the console's event stream is read over a shoulder and kept in
/// scrollback, so by default it says what happened and where, and never
/// what the User wrote or what a model answered.
#[test]
fn the_event_stream_prints_one_line_per_record_and_no_payload() {
    let line = printed(&dispatched(), Records::Summary).unwrap();
    assert_eq!(line, "  seq 42  steer_received  lab/room1");
}

/// The whole record stays one switch away, in the shape `sprawling call`
/// prints.
#[test]
fn the_whole_record_is_printed_only_when_asked_for() {
    let line = printed(&dispatched(), Records::Whole).unwrap();
    assert!(line.contains("the secret plan"), "{line}");
}
