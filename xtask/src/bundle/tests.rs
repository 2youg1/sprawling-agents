// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::{DECLARATION, declared, dist, name, restated};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level under the repository root")
        .to_path_buf()
}

fn items(source: &str) -> Vec<syn::Item> {
    syn::parse_file(source).unwrap().items
}

/// The reading this module exists for: the name comes out of the build
/// script rather than out of a copy kept here.
#[test]
fn the_name_is_read_from_the_file_that_embeds_the_bundle() {
    let stated = name(&root()).unwrap();
    assert!(!stated.is_empty(), "the build script states no directory");
    assert!(
        !stated.contains('/'),
        "the declaration names a directory, not a path: {stated}"
    );
    assert_eq!(dist(&root()).unwrap(), root().join("target").join(&stated));
}

/// A constant of another name is not this one, and a build script that
/// lost the declaration is a gate failure rather than a guess.
#[test]
fn only_the_named_declaration_answers() {
    assert_eq!(
        declared(&items(&format!(
            "const {DECLARATION}: &str = \"web-dist\";"
        ))),
        Some("web-dist".to_owned())
    );
    assert_eq!(declared(&items("const OTHER: &str = \"web-dist\";")), None);
    assert!(name(Path::new("/nonexistent-checkout")).is_err());
}

/// Both restating files name the directory the product embeds today.
#[test]
fn the_repository_itself_passes_the_check_it_ships() {
    let found = restated(&root()).unwrap();
    assert!(found.is_empty(), "{found:#?}");
}
