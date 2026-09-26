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
