// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the workspace's own packages are pinned to: the workspace's
//! version, written once per package in the root manifest, and nowhere
//! else by path.

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

/// The root manifest as this workspace writes it: a version, one
/// external dependency, and two of its own packages pinned to it.
const WORKSPACE: &str = r#"
[workspace.package]
version = "0.0.8"

[workspace.dependencies]
serde = "1"
kernel = { package = "sprawling-kernel", path = "crates/kernel", version = "=0.0.8" }
wire = { package = "sprawling-wire", path = "crates/wire", version = "=0.0.8", default-features = false }
"#;

/// A member taking both through the workspace table.
const MEMBER: &str = r#"
[package]
name = "sprawling-k"

[dependencies]
kernel = { workspace = true }
serde = { workspace = true }
"#;

fn judged(workspace: &str, member: &str) -> Vec<Violation> {
    pins(&read(workspace), &[("crates/k".to_owned(), read(member))])
}

#[test]
fn pins_that_name_the_workspace_version_are_quiet() {
    let found = judged(WORKSPACE, MEMBER);
    assert!(found.is_empty(), "{found:#?}");
}

/// The defect this exists for: the version moved and one pin did not,
/// which cargo reports as a resolution failure far from its cause.
#[test]
fn a_pin_left_behind_by_a_version_bump_is_named() {
    let behind = WORKSPACE.replace("=0.0.8\", default", "=0.0.7\", default");
    let found = judged(&behind, MEMBER);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].location,
        "Cargo.toml [workspace.dependencies] wire"
    );
    assert!(found[0].violation.contains("=0.0.7"), "{found:#?}");
    assert!(
        found[0].alternative.contains("version = \"=0.0.8\""),
        "{found:#?}"
    );
}

/// A path dependency without a version cannot be published at all.
#[test]
fn a_path_dependency_without_a_version_is_named() {
    let unpinned = WORKSPACE.replace(
        "path = \"crates/kernel\", version = \"=0.0.8\"",
        "path = \"crates/kernel\"",
    );
    let found = judged(&unpinned, MEMBER);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].location,
        "Cargo.toml [workspace.dependencies] kernel"
    );
    assert!(found[0].violation.contains("absent"), "{found:#?}");
}

/// A member that spells a path of its own is a second home for where a
/// package lives and what version it is, in whichever table it writes.
#[test]
fn a_member_that_names_a_workspace_package_by_path_is_named() {
    let own =
        format!("{MEMBER}\n[target.'cfg(windows)'.dependencies]\nj = {{ path = \"../j\" }}\n");
    let found = judged(WORKSPACE, &own);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].location,
        "crates/k/Cargo.toml [target.'cfg(windows)'.dependencies] j"
    );
    assert!(
        found[0].alternative.contains("j = { workspace = true }"),
        "{found:#?}"
    );
}
