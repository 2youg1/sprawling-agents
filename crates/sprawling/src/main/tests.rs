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

use super::data::verified_chain;
use super::router::{COMMANDS, named};
use super::{CLIENT_COMPLETE, CLIENT_FILES};

/// A verb the binary accepts is on the one screen that lists them. What
/// `doctor` reads off its own line is judged beside it, in the library.
#[test]
fn doctor_is_on_the_command_screen() {
    assert!(
        COMMANDS.contains("doctor"),
        "a verb the binary accepts is on the one screen that lists them"
    );
}

/// A place with no ledger in it is not a verified chain.
///
/// `replay` used to answer `chain verified: 0 line(s), tail seq none`
/// and exit 0 for an empty directory and for a city directory alike,
/// because a ledger with no events reads the same way. A scripted
/// integrity check pointed at the wrong argument scored a pass.
#[test]
fn a_place_holding_no_ledger_is_refused_rather_than_verified() {
    let dir = tempfile::tempdir().unwrap();
    let err = verified_chain(dir.path()).unwrap_err();
    assert_eq!(err.code(), &kernel::AxCode::PathNotFound);
    assert!(
        err.recovery().contains("ledger directory itself"),
        "the recovery names the mistake that was actually made: {}",
        err.recovery()
    );
}

/// A flag is not a path.
///
/// `sprawling init --help` used to raise a city in a directory
/// called `--help`, because `init` read `args[1]` whatever it was.
/// This repository's own root held one of those for a day.
#[test]
fn a_flag_is_never_read_as_the_path_a_subcommand_wanted() {
    let words =
        |raw: &[&str]| -> Vec<String> { raw.iter().map(|word| (*word).to_owned()).collect() };
    let asked = words(&["init", "--help"]);
    assert_eq!(named(&asked, 1), None, "--help became a city directory");

    let two = words(&["export", "--verbose", "city", "bundle"]);
    assert_eq!(named(&two, 1).map(String::as_str), Some("city"));
    assert_eq!(named(&two, 2).map(String::as_str), Some("bundle"));

    let plain = words(&["serve", "city", "127.0.0.1:8787"]);
    assert_eq!(named(&plain, 1).map(String::as_str), Some("city"));
    assert_eq!(named(&plain, 2).map(String::as_str), Some("127.0.0.1:8787"));
}

/// The embed chain delivers a file table with the page shell in it;
/// when the wasm client was built, the table carries it too.
#[test]
fn embedded_client_table_is_present_and_marked() {
    let index = CLIENT_FILES
        .iter()
        .find(|f| f.path == "index.html")
        .expect("the page shell is always embedded");
    assert!(!index.gz.is_empty());
    if CLIENT_COMPLETE {
        // Named by their directory rather than by file name: every chunk
        // the bundler writes carries a content hash, so the names change
        // on every build and only the shape of the path is stable.
        assert!(
            CLIENT_FILES
                .iter()
                .any(|f| f.path.starts_with("assets/") && f.path.ends_with(".js")),
            "a complete client carries a script"
        );
        assert!(
            CLIENT_FILES
                .iter()
                .any(|f| f.path.starts_with("assets/") && f.path.ends_with(".css")),
            "a complete client carries a stylesheet"
        );
    }
}
