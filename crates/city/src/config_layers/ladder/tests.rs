// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError};

use super::Ladder;
use crate::Layer;
use crate::config_layers::path;
use crate::config_layers::refuse::two_residents;

/// A layer the parser refuses is refused on the ladder with its file
/// named and the parser's own recovery kept: the dispatch path is told
/// `/new` is the way out, as `sprawling check` is (city-SPEC 12.8 (a)).
#[test]
fn a_layer_the_parser_refuses_keeps_the_parsers_recovery_on_the_ladder() {
    let dir = tempfile::tempdir().unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let file = path(dir.path(), &room, Layer::Building).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(
        &file,
        "[model]\nname = \"m-local\"\n\n[resident]\nharness = \"pi\"\n",
    )
    .unwrap();
    let parsed = two_residents("m-local", "pi");

    assert_eq!(
        Ladder::read(dir.path(), &room),
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read a configuration layer",
            format!("{}: {}", file.display(), parsed.subject()),
        )
        .with_recovery(parsed.recovery()))
    );
}
