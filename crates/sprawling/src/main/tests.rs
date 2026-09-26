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
use super::{CLIENT_BUNDLE_DIR, CLIENT_COMPLETE, CLIENT_FILES};

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

/// The binary embeds the bundle `just build-web` wrote into the
/// workspace, wherever cargo puts its own output.
///
/// With `CARGO_TARGET_DIR` pointing outside the workspace, the build
/// script used to look for the bundle there, found nothing, and shipped
/// the placeholder page beside a real bundle it never read. Under the
/// default target directory both places coincide, so this test can only
/// tell the two readings apart where the variable is set.
#[test]
fn the_embedded_client_is_the_bundle_the_workspace_built() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(CLIENT_BUNDLE_DIR);
    let mut on_disk = Vec::new();
    files_under(&dist, &dist, &mut on_disk);
    on_disk.sort();
    let built = on_disk.iter().any(|path| path == "index.html")
        && on_disk.iter().any(|path| path.starts_with("assets/"));
    let embedded: Vec<String> = CLIENT_FILES
        .iter()
        .filter(|_| CLIENT_COMPLETE)
        .map(|file| file.path.to_owned())
        .collect();
    let expected = if built { on_disk } else { Vec::new() };
    assert_eq!(
        embedded,
        expected,
        "the binary embeds exactly the bundle at {}",
        dist.display()
    );
}

fn files_under(root: &std::path::Path, dir: &std::path::Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(root, &path, found);
        } else {
            let relative = path.strip_prefix(root).unwrap();
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// `call`'s rows of the exit-code table: each way a call can end
/// reaches its own code, and none of these touch a city that exists.
///
/// A frame the wire cannot carry is this command line's fault (2), and
/// an address where nothing answers is no city at all (4); both used to
/// exit 1, which an agent reads as "the city refused" and answers by
/// fixing a frame the city never saw.
#[test]
fn each_way_a_call_ends_has_its_own_exit_code() {
    use super::calling::call;
    use super::exit::Exit;
    const CITY_VIEW: &str = r#"{"ask":{"ask_id":1,"query":"city_view"}}"#;
    const NO_SUCH_QUERY: &str = r#"{"ask":{"ask_id":1,"query":"no_such"}}"#;
    // A port that was bound and released: nothing listens on it.
    let vacant = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    let words = |line: &[&str]| line.iter().map(ToString::to_string).collect::<Vec<_>>();
    let table = [
        (words(&["call"]), Exit::Line),
        (words(&["call", "{not json", "--at", &vacant]), Exit::Line),
        (words(&["call", NO_SUCH_QUERY, "--at", &vacant]), Exit::Line),
        (
            words(&["call", CITY_VIEW, "--quiet-ms", "soon"]),
            Exit::Line,
        ),
        (
            words(&["call", CITY_VIEW, "--until", "no_such_kind"]),
            Exit::Line,
        ),
        (words(&["call", CITY_VIEW, "--at", &vacant]), Exit::NoCity),
    ];
    let observed = table
        .iter()
        .map(|(line, _)| (line.clone(), call(line)))
        .collect::<Vec<_>>();
    assert_eq!(observed, table.to_vec());
}

/// A refusal names the nearby names to a person, and reaches a program
/// as the `AxError` it was, so neither reader loses a field.
#[test]
fn a_refusal_reads_the_same_to_a_person_and_to_a_program() {
    use super::refusal::{Form, written};
    use kernel::{AxCode, AxError};
    let err = AxError::failure(AxCode::ToolUnknown, "call tool", "grep")
        .with_nearby(vec!["exec".into(), "edit".into()])
        .with_recovery("use one of the nearby tools");
    let human = written(&err, Form::Human);
    let json = written(&err, Form::Json);
    let back: AxError = serde_json::from_str(&json).unwrap();
    assert_eq!(
        (human.lines().last(), json.lines().count(), back),
        (Some("nearby: exec, edit"), 1, err)
    );
}
