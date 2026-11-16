// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A city the previous release wrote, opened by this build after the
//! move home (`crates/sprawling/spec/Firstrun.lean` §8-8).
//!
//! `tests/cities/v0.0.12` is what the 0.0.12 release binary wrote with
//! `sprawling init city` beside itself and then served once, kept byte
//! for byte (`.gitattributes` turns off conversion for it). Three names
//! are spelled differently in the tree so that this repository's git
//! does not read them as its own: `dot-git` is the city's `.git`, and
//! each `dot-gitignore` is a `.gitignore` the city wrote.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::Path;

use sprawling::beside_binary::{Homing, bring_home};

/// The fixture's spelling of each name and the name the city wrote.
const RESPELLED: [(&str, &str); 2] = [("dot-git", ".git"), ("dot-gitignore", ".gitignore")];

/// Copies the fixture to `to`, putting back the names it respells.
fn unpack(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        let name = RESPELLED
            .iter()
            .find(|(respelled, _)| *respelled == name)
            .map_or(name.as_str(), |(_, written)| written)
            .to_owned();
        if entry.file_type().unwrap().is_dir() {
            unpack(&entry.path(), &to.join(name));
        } else {
            std::fs::copy(entry.path(), to.join(name)).unwrap();
        }
    }
}

#[test]
fn a_city_the_previous_release_left_beside_its_binary_opens_after_the_move_home() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = dir.path().join("sprawling-0.0.12-windows-x86_64");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cities")
        .join("v0.0.12");
    unpack(&fixture, &unpacked.join(accounting::home::CITY_DIR));
    let home = accounting::home::Home::at(dir.path().join("person"));

    let moved = bring_home(&unpacked, &home.default_city()).unwrap();
    assert!(
        matches!(moved, Homing::Moved { .. }),
        "the city beside the binary moves: {moved:?}"
    );
    let city_root = home.default_city();

    let report = city::check(&city_root).unwrap();
    assert!(
        report.findings.is_empty(),
        "every file the previous release wrote is readable: {:?}",
        report.findings
    );
    assert!(report.read > 0, "the check read the city's files");

    accounting::worker::RunWorker::new(
        &city_root,
        runtime::diagnostics::Diagnostics::off(),
        sprawling::assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
}
