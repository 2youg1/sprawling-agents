// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the wall comparison is held to: a copied lint table that has
//! been edited on one side, a version line that has moved, a recorded
//! difference that is no longer a difference, and this repository's own
//! two manifests.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;

fn read(text: &str) -> toml::Value {
    toml::from_str(text).unwrap()
}

const WORKSPACE: &str = r#"
[workspace.package]
version = "0.0.5"
edition = "2024"
license = "MPL-2.0"
rust-version = "1.97"
publish = false

[workspace.dependencies]
serde_json = "1"
toml = "1.1"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
as_conversions = "deny"
"#;

const DESKTOP: &str = r#"
[package]
version = "0.0.5"
edition = "2024"
license = "MPL-2.0"
rust-version = "1.97"
publish = false

[dependencies]
serde_json = "1"
toml = "1.1"
image = { version = "0.25", default-features = false }

[lints.rust]
unsafe_code = "deny"

[lints.clippy]
unwrap_used = "deny"
as_conversions = "deny"
"#;

fn compared(workspace: &str, desktop: &str) -> Vec<Violation> {
    let (workspace, desktop) = (read(workspace), read(desktop));
    let mut out = Vec::new();
    metadata(&workspace, &desktop, &mut out);
    lints(&workspace, &desktop, &mut out);
    dependencies(&workspace, &desktop, &mut out);
    out
}

/// The pair as it stands: one recorded difference, and nothing else.
#[test]
fn two_walls_that_differ_only_where_somebody_decided_are_quiet() {
    assert!(compared(WORKSPACE, DESKTOP).is_empty());
}

/// The defect this gate exists for. A clippy lint dropped from the
/// out-of-tree copy silently un-enforces it for 5,800 lines that no
/// workspace command compiles.
#[test]
fn a_lint_the_copy_no_longer_carries_is_refused_and_named() {
    let weakened = DESKTOP.replace("as_conversions = \"deny\"", "");
    let found = compared(WORKSPACE, &weakened);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].location.contains("as_conversions"), "{found:#?}");
    assert!(found[0].violation.contains("absent"), "{found:#?}");
    assert!(found[0].alternative.contains("RECORDED"), "{found:#?}");
}

/// A lint relaxed rather than removed is the same finding, and a lint
/// the copy adds alone is one too: a wall nobody else stands behind is
/// a rule with one home too many.
#[test]
fn a_lint_relaxed_on_one_side_and_a_lint_added_on_one_side_are_both_refused() {
    let relaxed = DESKTOP.replace("unwrap_used = \"deny\"", "unwrap_used = \"warn\"");
    let found = compared(WORKSPACE, &relaxed);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].violation.contains("warn"), "{found:#?}");

    let added = format!("{DESKTOP}\nstring_slice = \"deny\"\n");
    let found = compared(WORKSPACE, &added);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].location.contains("string_slice"), "{found:#?}");
}

/// A recorded difference that has stopped being one is struck, the way
/// a length register entry is struck when its file comes back under
/// budget. The refusal repeats the reason the row was granted for.
#[test]
fn a_recorded_difference_the_two_sides_now_agree_on_is_struck() {
    let matched = DESKTOP.replace("unsafe_code = \"deny\"", "unsafe_code = \"forbid\"");
    let found = compared(WORKSPACE, &matched);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].location.contains("RECORDED"), "{found:#?}");
    assert!(found[0].alternative.contains("Win32"), "{found:#?}");
}

/// Metadata is inherited inside the workspace and typed out here, so a
/// version bump that reaches the workspace alone is caught.
#[test]
fn metadata_that_stayed_behind_a_version_bump_is_caught() {
    let bumped = WORKSPACE.replace("version = \"0.0.5\"", "version = \"0.0.6\"");
    let found = compared(&bumped, DESKTOP);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].location.contains("[package] version"),
        "{found:#?}"
    );
}

/// A dependency both manifests name is one version line. A dependency
/// only this package names is its own business, and the comparison says
/// nothing about it.
#[test]
fn a_shared_dependency_is_held_to_one_version_and_a_private_one_is_not() {
    let drifted = DESKTOP.replace("toml = \"1.1\"", "toml = \"0.8\"");
    let found = compared(WORKSPACE, &drifted);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].violation.contains("`0.8`"), "{found:#?}");
    assert!(found[0].alternative.contains("1.1"), "{found:#?}");
    // `image` is named by one manifest only, and stays unjudged.
    assert!(
        compared(WORKSPACE, &DESKTOP.replace("\"0.25\"", "\"0.24\"")).is_empty(),
        "a dependency the workspace does not name is this package's own"
    );
}

/// Both copied constants are read out of real source, so the reader is
/// the shape the source is written in rather than a regular expression
/// somebody hoped would match it.
#[test]
fn the_quoted_facts_are_read_out_of_the_files_that_hold_them() {
    let root = repository();
    assert_eq!(
        revision(&root, PROTOCOL_HOME).unwrap(),
        revision(&root, PROTOCOL_COPY).unwrap()
    );
    let defined = codes(&root, CODE_HOME).unwrap();
    let quoted = codes(&root, CODE_QUOTE).unwrap();
    assert!(quoted.contains("E_GATE_DENIED"), "{quoted:?}");
    assert!(quoted.is_subset(&defined), "{quoted:?}");
}

/// The repository passes the check it ships. A gate whose own tree is
/// red teaches people that red is the normal colour.
#[test]
fn the_repository_itself_passes_the_check_it_ships() {
    let found = check(&repository()).unwrap();
    assert!(found.is_empty(), "{found:#?}");
}

fn repository() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .expect("xtask lives one level under the repo root")
}
