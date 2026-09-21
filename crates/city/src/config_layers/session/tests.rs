// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The record a session writes at its own address, read back as it was
//! written.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;
use crate::config_layers::{Layer, settled_effort};

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

/// A session's record round-trips through the file grammar: what
/// `write_session` states is what `own_layer` reads back, and the model
/// is stated even when the person chose no effort.
///
/// The record is one act because the two values are one choice, so the
/// round trip is one test: a write that lost either half would leave a
/// later run reading half a shape.
#[test]
fn a_session_record_is_read_back_exactly_as_it_was_written() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    assert_eq!(own_layer(dir.path(), &room).unwrap().model(), None);

    write_session(dir.path(), &room, "m-local", Some(Effort::High)).unwrap();
    let recorded = own_layer(dir.path(), &room).unwrap();
    assert_eq!(recorded.model(), Some("m-local"));
    assert_eq!(recorded.effort(), Some(Effort::High));
    assert_eq!(
        settled_effort(dir.path(), &room).unwrap().map(|(e, _)| e),
        Some(Effort::High),
        "and the ladder every run climbs answers with it"
    );

    // No effort chosen is not the same fact as an effort of `none`: a
    // room whose person chose none keeps the key out, and the ladder
    // goes on answering nothing for it.
    let fresh = addr("lab/fresh");
    write_session(dir.path(), &fresh, "m-local", None).unwrap();
    let stated =
        std::fs::read_to_string(path(dir.path(), &fresh, Layer::Resident).unwrap()).unwrap();
    assert!(
        stated.contains("name = \"m-local\"") && !stated.contains("effort"),
        "{stated}"
    );
    assert_eq!(settled_effort(dir.path(), &fresh).unwrap(), None);
    assert_eq!(
        own_layer(dir.path(), &fresh).unwrap().model(),
        Some("m-local")
    );

    // An empty name states nothing at all, which is a mistake in the
    // file rather than a model: it is refused where it is read.
    assert!(
        ConfigLayer::parse(
            "[model]
name = \"\"
"
        )
        .is_err()
    );
    assert!(
        ConfigLayer::parse(
            "[model]
model = \"m-local\"
"
        )
        .is_err()
    );
}
