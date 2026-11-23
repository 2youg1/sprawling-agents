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

use super::*;
use crate::config_layers::{Layer, own_layer, path, settled_effort, write_session};
use kernel::Effort;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

fn room_with_a_frozen_shape(city_root: &Path, room: &Address) -> String {
    write_session(
        city_root,
        room,
        &crate::SessionShape::new("m-local", Some(Effort::High), None),
    )
    .unwrap();
    let file = path(city_root, room, Layer::Resident).unwrap();
    let mut held = std::fs::read_to_string(&file).unwrap();

    held.push_str("\n[sandbox]\nshell = true\n");
    std::fs::write(&file, &held).unwrap();
    held
}

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
        after.contains("shell = true"),
        "a key the person wrote is not this session's to forget: {after}"
    );
    assert!(!after.contains("m-local"), "{after}");
}

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

    clear_session(dir.path(), &room).unwrap();
}

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
