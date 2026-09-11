// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use super::{ROWS, npm_version};

/// The last version whose platform packages reached the registry under
/// bare names. npm never reuses a `name@version`, so the scope cannot
/// be applied to it afterwards.
const UNSCOPED_THROUGH: &str = "0.0.4";

#[test]
fn a_release_tag_becomes_the_semver_npm_accepts() {
    assert_eq!(
        npm_version("v0.0.4-Pre-alpha-260911", "0.0.4").unwrap(),
        "0.0.4-pre.260911"
    );
}

/// The check that keeps one release from having two version numbers.
#[test]
fn a_tag_disagreeing_with_the_workspace_is_refused() {
    let err = npm_version("v0.0.3-Pre-alpha-260911", "0.0.4").unwrap_err();
    assert!(err.to_string().contains("two version numbers"), "{err}");
}

#[test]
fn a_tag_of_another_shape_is_refused_rather_than_guessed_at() {
    assert!(npm_version("0.0.4", "0.0.4").is_err());
    assert!(npm_version("v0.0.4", "0.0.4").is_err());
    assert!(npm_version("v0.0.4-Pre-alpha-26091", "0.0.4").is_err());
    assert!(npm_version("v0.0.4-Pre-alpha-2609xx", "0.0.4").is_err());
}

/// Two packages claiming one platform would make which binary a
/// person gets depend on the order the assets were read in.
#[test]
fn no_two_rows_claim_one_platform_or_one_suffix() {
    for (index, row) in ROWS.iter().enumerate() {
        for other in ROWS.iter().skip(index + 1) {
            assert_ne!((row.os, row.cpu), (other.os, other.cpu));
            assert_ne!(row.suffix, other.suffix);
            assert_ne!(row.package, other.package);
        }
    }
}

/// The decision beside `ROWS`, carried to the version that can honour
/// it. Nothing here fires while the workspace still states the version
/// whose packages are already published; the first commit that moves
/// that number turns this red, and the message says what to rename.
#[test]
fn the_platform_packages_take_the_scope_from_the_next_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one directory under the workspace root");
    let version = crate::package::workspace_version(root).expect("the workspace states a version");
    if version == UNSCOPED_THROUGH {
        return;
    }
    for row in ROWS {
        assert!(
            row.package.starts_with("@sprawling/"),
            "the workspace states {version}, which is past {UNSCOPED_THROUGH}, \
             so {} belongs under the @sprawling scope. Rename it here and in \
             the shim's table; the root package keeps its bare name, because \
             `bunx sprawling` is what this channel exists for.",
            row.package
        );
    }
}

/// Found by running it rather than by reading it: npm's generated
/// wrapper reads the shebang to decide what interprets the file, and
/// without one Windows ran the JavaScript as a shell script, printed
/// nothing, and exited 0. A `bunx sprawling` that reports success
/// and does nothing is the worst shape this channel can fail in, so
/// the first line is held here.
#[test]
fn the_shim_begins_with_a_shebang() {
    assert!(
        super::SHIM.starts_with("#!/usr/bin/env node\n"),
        "without a shebang npm's wrapper runs this as a shell script"
    );
}

/// The shim is what every root package carries, so its own contract
/// is worth holding: it resolves a platform package and never
/// installs anything.
#[test]
fn the_shim_execs_rather_than_installs() {
    let shim = super::SHIM;
    assert!(shim.contains("require.resolve"), "the shim must resolve");
    assert!(
        !shim.contains("sprawling install"),
        "PATH belongs to whoever installed; the shim must not install"
    );
    for row in ROWS {
        assert!(
            shim.contains(row.package),
            "the shim does not know {}",
            row.package
        );
    }
}
