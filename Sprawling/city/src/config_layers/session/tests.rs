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
use crate::config_layers::{Layer, settled_effort};

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

#[test]
fn a_session_record_is_read_back_exactly_as_it_was_written() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    assert_eq!(own_layer(dir.path(), &room).unwrap().model(), None);

    let mut shape = crate::SessionShape::new("m-local", Some(Effort::High), None);
    shape.endpoint = Some("house".to_owned());
    write_session(dir.path(), &room, &shape).unwrap();
    let recorded = own_layer(dir.path(), &room).unwrap();
    assert_eq!(recorded.model(), Some("m-local"));
    assert_eq!(recorded.endpoint(), Some("house"));
    assert_eq!(recorded.effort(), Some(Effort::High));
    assert_eq!(
        settled_effort(dir.path(), &room).unwrap().map(|(e, _)| e),
        Some(Effort::High),
        "and the ladder every run climbs answers with it"
    );

    let fresh = addr("lab/fresh");
    write_session(
        dir.path(),
        &fresh,
        &crate::SessionShape::new("m-local", None, None),
    )
    .unwrap();
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
