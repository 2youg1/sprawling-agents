// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Starting a session: what a room forgets, and what it keeps.
//!
//! Every test here writes a room the way a person would have it — the
//! session's own record plus keys a person typed — and then asks what
//! survives. The keys a person wrote are the point: a clearing that
//! rewrote the file from what this build understands would take them
//! with it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;
use crate::config_layers::{Layer, own_layer, path, settled_effort, write_session};
use kernel::Effort;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

/// A room whose first session froze a shape, with a key a person typed
/// beside it.
fn room_with_a_frozen_shape(city_root: &Path, room: &Address) -> String {
    write_session(city_root, room, "m-local", Some(Effort::High)).unwrap();
    let file = path(city_root, room, Layer::Resident).unwrap();
    let mut held = std::fs::read_to_string(&file).unwrap();
    // A key in the same file that no session wrote: the sandbox is the
    // person stating what a run in this room may reach, and a cleared
    // session is not allowed to take it with it.
    held.push_str("\n[sandbox]\nfuel = 900\n");
    std::fs::write(&file, &held).unwrap();
    held
}

/// The question `/new` answers: after it, the room chooses its own
/// shape again, and every key the person wrote is still there.
#[test]
fn clearing_a_session_forgets_the_shape_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let before = room_with_a_frozen_shape(dir.path(), &room);
    assert!(before.contains("name = \"m-local\""));

    clear_session(dir.path(), &room).unwrap();

    let own = own_layer(dir.path(), &room).unwrap();
    assert_eq!(
        own.model(),
        None,
        "the model is the person's to choose again"
    );
    assert_eq!(own.effort(), None);
    assert_eq!(settled_effort(dir.path(), &room).unwrap(), None);
    let after = std::fs::read_to_string(path(dir.path(), &room, Layer::Resident).unwrap()).unwrap();
    assert!(
        after.contains("fuel = 900"),
        "a key the person wrote is not this session's to forget: {after}"
    );
    assert!(!after.contains("m-local"), "{after}");
}

/// The half `--carry` keeps: the shape goes and the summary stays.
///
/// The two halves are asserted together because the pair is the ruling:
/// a person who carries the handoff still gets to choose the model again,
/// which is what makes the room dispatchable after they changed it.
#[test]
fn carrying_the_handoff_still_forgets_the_shape() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    room_with_a_frozen_shape(dir.path(), &room);
    let handoff = crate::spine_files::handoff_path(dir.path(), &room);
    std::fs::create_dir_all(handoff.parent().unwrap()).unwrap();
    std::fs::write(&handoff, "# what the last session left\n\nit went well\n").unwrap();

    forget_shape(dir.path(), &room).unwrap();

    assert_eq!(own_layer(dir.path(), &room).unwrap().model(), None);
    assert_eq!(
        crate::spine_files::handoff(dir.path(), &room).unwrap(),
        Some("# what the last session left\n\nit went well\n".to_owned()),
        "the summary is what the person asked to carry"
    );
}

/// A new session carries nothing, and the slot is emptied rather than
/// annotated.
#[test]
fn clearing_a_session_empties_the_handoff_slot() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let handoff = crate::spine_files::handoff_path(dir.path(), &room);
    std::fs::create_dir_all(handoff.parent().unwrap()).unwrap();
    std::fs::write(&handoff, "the previous session's summary\n").unwrap();
    assert!(
        crate::spine_files::handoff(dir.path(), &room)
            .unwrap()
            .is_some()
    );

    clear_session(dir.path(), &room).unwrap();

    assert_eq!(
        crate::spine_files::handoff(dir.path(), &room).unwrap(),
        None
    );
    assert!(!handoff.exists(), "the slot is emptied, not blanked");
    // And clearing a room that never had one is not a failure: the person
    // asked for a new session, which is a thing that can be done.
    clear_session(dir.path(), &room).unwrap();
}

/// A configuration this build cannot understand is refused rather than
/// overwritten, so a person's file survives a `/new` typed at the wrong
/// moment.
#[test]
fn a_file_that_does_not_parse_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let file = path(dir.path(), &room, Layer::Resident).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "[model\nname = broken\n").unwrap();

    let refused = clear_session(dir.path(), &room).unwrap_err();

    assert_eq!(refused.code(), &kernel::AxCode::ConfigInvalid);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "[model\nname = broken\n"
    );
}
