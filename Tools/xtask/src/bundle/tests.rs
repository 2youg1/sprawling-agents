// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::{DECLARATION, declared, dist, stated_path};

fn root() -> PathBuf {
    crate::root::this_checkout().to_path_buf()
}

fn items(source: &str) -> Vec<syn::Item> {
    syn::parse_file(source).unwrap().items
}

#[test]
fn the_path_is_read_from_the_file_that_embeds_the_bundle() {
    let stated = stated_path(&root()).unwrap();
    assert!(!stated.is_empty(), "the build script states no path");
    let found = dist(&root()).unwrap();
    let under_root: Vec<String> = found
        .strip_prefix(root())
        .expect("the bundle lands under the workspace root")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(under_root.join("/"), stated);
}

#[test]
fn only_the_named_declaration_answers() {
    assert_eq!(
        declared(&items(&format!(
            "const {DECLARATION}: &str = \"web-dist\";"
        ))),
        Some("web-dist".to_owned())
    );
    assert_eq!(declared(&items("const OTHER: &str = \"web-dist\";")), None);
    assert!(stated_path(Path::new("/nonexistent-checkout")).is_err());
}
