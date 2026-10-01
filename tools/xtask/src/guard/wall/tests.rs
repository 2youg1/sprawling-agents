// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the one lint table of its own is held to: a lint dropped,
//! relaxed or added on the leaf's side, a recorded difference that is no
//! longer a difference, another member writing a table of its own, and a
//! leaf that is no longer a member at all.

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
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
as_conversions = "deny"
"#;

/// The leaf as `crates/desktop/ffi/Cargo.toml` writes it: the
/// workspace's table, with the one recorded line changed.
const LEAF_MANIFEST: &str = r#"
[package]
name = "sprawling-desktop-ffi"

[lints.rust]
unsafe_code = "deny"

[lints.clippy]
unwrap_used = "deny"
as_conversions = "deny"
"#;

/// Any other member, inheriting the workspace's table.
const MEMBER: &str = r#"
[package]
name = "sprawling-k"

[lints]
workspace = true
"#;

fn judged(leaf: &str, member: &str) -> Vec<Violation> {
    tables(
        &read(WORKSPACE),
        &[
            ("crates/k".to_owned(), read(member)),
            (LEAF.to_owned(), read(leaf)),
        ],
    )
}

/// The pair as it stands: one recorded difference, and nothing else.
#[test]
fn a_leaf_that_differs_only_where_somebody_decided_is_quiet() {
    let found = judged(LEAF_MANIFEST, MEMBER);
    assert!(found.is_empty(), "{found:#?}");
}

/// The defect this gate exists for. A clippy lint dropped from the
/// leaf's table silently un-enforces it in the one crate that may write
/// `unsafe`.
#[test]
fn a_lint_the_leaf_no_longer_carries_is_refused_and_named() {
    let weakened = LEAF_MANIFEST.replace("as_conversions = \"deny\"", "");
    let found = judged(&weakened, MEMBER);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].location,
        "crates/desktop/ffi/Cargo.toml [lints.clippy] as_conversions"
    );
    assert!(found[0].violation.contains("absent"), "{found:#?}");
    assert!(found[0].alternative.contains("RECORDED"), "{found:#?}");
}

/// A lint relaxed rather than removed is the same finding, and a lint
/// the leaf adds alone is one too: a rule only one crate stands behind
/// is a rule with one home too many.
#[test]
fn a_lint_relaxed_on_one_side_and_a_lint_added_on_one_side_are_both_refused() {
    let relaxed = LEAF_MANIFEST.replace("unwrap_used = \"deny\"", "unwrap_used = \"warn\"");
    let found = judged(&relaxed, MEMBER);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].violation.contains("warn"), "{found:#?}");

    let added = LEAF_MANIFEST.replace(
        "as_conversions = \"deny\"",
        "as_conversions = \"deny\"\nstring_slice = \"deny\"",
    );
    let found = judged(&added, MEMBER);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].location.contains("string_slice"), "{found:#?}");
}

/// A recorded difference that has stopped being one is struck, the way
/// a length register entry is struck when its file comes back under
/// budget. The refusal repeats the reason the row was granted for.
#[test]
fn a_recorded_difference_the_two_sides_now_agree_on_is_struck() {
    let matched = LEAF_MANIFEST.replace("unsafe_code = \"deny\"", "unsafe_code = \"forbid\"");
    let found = judged(&matched, MEMBER);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].location.contains("RECORDED"), "{found:#?}");
    assert!(found[0].alternative.contains("Zig leaf"), "{found:#?}");
}

/// Any other member that writes a table of its own stands outside the
/// comparison, so it is named at its own manifest.
#[test]
fn another_member_with_a_table_of_its_own_is_named() {
    let own = MEMBER.replace(
        "[lints]\nworkspace = true",
        "[lints.rust]\nunsafe_code = \"deny\"",
    );
    let found = judged(LEAF_MANIFEST, &own);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].location, "crates/k/Cargo.toml [lints]");
    assert!(
        found[0].alternative.contains("workspace = true"),
        "{found:#?}"
    );
}

/// A leaf that is no longer a member leaves its exception granted to
/// nothing; the exception is struck with it.
#[test]
fn a_leaf_that_is_no_longer_a_member_strikes_its_exception() {
    let found = tables(
        &read(WORKSPACE),
        &[("crates/k".to_owned(), read(MEMBER))],
    );
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].location.contains("LEAF"), "{found:#?}");
    assert!(found[0].alternative.contains("RECORDED"), "{found:#?}");
}
