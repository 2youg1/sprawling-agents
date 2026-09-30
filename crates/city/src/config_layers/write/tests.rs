// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the four write faces state, and what they refuse to write.
//!
//! The second rung's domain is `SecondThreshold`'s one construction
//! point and is tested there; what is tested here is that a taken value
//! lands where the ladder reads it back.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;

fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

fn hosted(headers: Vec<(String, String)>) -> Vec<McpServer> {
    vec![McpServer {
        label: ServerLabel::parse("hosted").unwrap(),
        transport: McpTransport::Http {
            url: "https://example.test/mcp".to_owned(),
            headers,
        },
    }]
}

/// A key typed into the header table of the settings page would be
/// written verbatim into a file the project commits, so the write
/// face refuses it and says where the value belongs instead.
/// The second rung survives a round trip: what the write face puts into
/// the layer's own file is what a later read of that file states.
#[test]
fn the_second_rung_is_written_into_the_layers_own_file() {
    let dir = tempfile::tempdir().unwrap();
    let threshold = SecondThreshold::parse(72).unwrap();
    write_second_threshold(dir.path(), &room(), Layer::Building, threshold).unwrap();
    let file = path(dir.path(), &room(), Layer::Building).unwrap();
    let document = read_document(&file).unwrap();
    assert_eq!(
        document["context"]["second_threshold"].as_integer(),
        Some(72),
        "the percent lands where the ladder reads it"
    );
}

/// Both ends of the domain are written. The domain itself is
/// `SecondThreshold`'s one construction point and is judged there, not
/// re-judged here.
#[test]
fn both_ends_of_the_domain_are_written() {
    for (percent, landed) in [(30, 30_i64), (90, 90_i64)] {
        let dir = tempfile::tempdir().unwrap();
        let threshold = SecondThreshold::parse(percent).unwrap();
        write_second_threshold(dir.path(), &room(), Layer::Building, threshold).unwrap();
        let file = path(dir.path(), &room(), Layer::Building).unwrap();
        let document = read_document(&file).unwrap();
        assert_eq!(
            document["context"]["second_threshold"].as_integer(),
            Some(landed)
        );
    }
}

#[test]
fn a_header_holding_a_credential_is_refused_before_the_file_is_touched() {
    let dir = tempfile::tempdir().unwrap();
    let servers = hosted(vec![(
        "Authorization".to_owned(),
        "Bearer sk-ant-not-a-real-key".to_owned(),
    )]);
    let err = write_mcp(dir.path(), &room(), Layer::City, &servers).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("Authorization"), "{}", err.subject());
    assert!(err.recovery().contains("secret:realm/name"));
    assert!(
        !path(dir.path(), &room(), Layer::City).unwrap().exists(),
        "a refused write left a file behind"
    );
}

/// One value at a time: the server whose other header is ordinary
/// is still refused for the one that is not.
#[test]
fn each_value_is_judged_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let servers = hosted(vec![
        ("X-Account".to_owned(), "acme".to_owned()),
        ("Authorization".to_owned(), "secret:mcp/hosted".to_owned()),
    ]);
    write_mcp(dir.path(), &room(), Layer::City, &servers).unwrap();

    let leaked = hosted(vec![
        ("X-Account".to_owned(), "acme".to_owned()),
        (
            "X-Trace".to_owned(),
            // Assembled here rather than written whole, so the
            // repository's own secret scanner does not read this
            // fixture as a leaked credential.
            format!("ghp_{}{}{}", "aB3dE5fG7hJ9k", "L1mN3pQ5rS7t", "U9vW1xY3zA5"),
        ),
    ]);
    let err = write_mcp(dir.path(), &room(), Layer::City, &leaked).unwrap_err();
    assert!(err.subject().contains("X-Trace"), "{}", err.subject());
}

/// An environment value is judged by the same rule as a header: the
/// table it sits in changes the noun in the refusal and nothing
/// else.
#[test]
fn a_credential_in_a_command_environment_is_refused_too() {
    let dir = tempfile::tempdir().unwrap();
    let servers = vec![McpServer {
        label: ServerLabel::parse("apps").unwrap(),
        transport: McpTransport::Stdio {
            command: "mcp-apps".to_owned(),
            args: Vec::new(),
            env: vec![("API_KEY".to_owned(), "plain-value".to_owned())],
        },
    }];
    let err = write_mcp(dir.path(), &room(), Layer::City, &servers).unwrap_err();
    assert!(
        err.subject().contains("environment value"),
        "{}",
        err.subject()
    );
}

/// An address that is its own building has one file for both rungs, so
/// a session record written into a building that names a harness would
/// leave a file every reader refuses. The write is refused instead, and
/// the file keeps the bytes it had.
#[test]
fn a_write_the_reader_would_refuse_is_not_written() {
    let dir = tempfile::tempdir().unwrap();
    let lab = Address::parse("lab").unwrap();
    let file = path(dir.path(), &lab, Layer::Resident).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let original = "[resident]\nharness = \"pi\"\n";
    std::fs::write(&file, original).unwrap();

    assert_eq!(
        super::super::write_session(dir.path(), &lab, "m-local", None),
        Err(refuse_file(
            &file,
            super::super::refuse::two_residents("m-local", "pi").subject()
        ))
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
}
