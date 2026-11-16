// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::members;
use crate::report::{Violation, XtaskError};
use crate::walk;

const MARKERS: [&str; 6] = [
    "conformance",
    "_for_test",
    "mock_",
    "fake_",
    "stub_",
    "dummy_",
];

const TEST_FEATURES: [&str; 3] = ["conformance", "fault", "fixture"];

const PRODUCT: &str = "Sprawling/app/Cargo.toml";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let files = members::product(root)?
        .iter()
        .map(|member| walk::files_with_ext(&root.join(&member.dir), &["rs"]))
        .collect::<Result<Vec<_>, _>>()?;
    for file in files.into_iter().flatten() {
        let rel = walk::rel(root, &file);
        if !rel.contains("/src/") {
            continue;
        }
        let text = walk::read_text(&file)?;
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        if has_cfg(&parsed.attrs) {
            continue;
        }
        walk_items(&rel, &parsed.items, &mut violations);
    }
    violations.extend(product_features(root)?);
    violations.extend(crate::bundle::restated(root)?);
    violations.extend(crate::platform::restated(root)?);
    violations.extend(crate::attestation::unattested(root)?);
    Ok(violations)
}

fn walk_items(rel: &str, items: &[syn::Item], out: &mut Vec<Violation>) {
    for item in items {
        let attrs = attrs_of(item);
        if has_cfg(attrs) {
            continue;
        }
        for name in names_of(item) {
            if let Some(marker) = MARKERS.iter().find(|marker| name.contains(*marker)) {
                out.push(ships(rel, &name, marker));
            }
        }
        if let syn::Item::Mod(module) = item
            && let Some((_, inner)) = &module.content
        {
            walk_items(rel, inner, out);
        }
    }
}

fn attrs_of(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Fn(it) => &it.attrs,
        syn::Item::Mod(it) => &it.attrs,
        syn::Item::Use(it) => &it.attrs,
        syn::Item::Struct(it) => &it.attrs,
        syn::Item::Enum(it) => &it.attrs,
        syn::Item::Const(it) => &it.attrs,
        syn::Item::Static(it) => &it.attrs,
        syn::Item::Trait(it) => &it.attrs,
        syn::Item::Type(it) => &it.attrs,
        syn::Item::Impl(it) => &it.attrs,
        _ => &[],
    }
}

fn has_cfg(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("cfg"))
}

fn names_of(item: &syn::Item) -> Vec<String> {
    match item {
        syn::Item::Fn(it) => vec![it.sig.ident.to_string()],
        syn::Item::Mod(it) => vec![it.ident.to_string()],
        syn::Item::Struct(it) => vec![it.ident.to_string()],
        syn::Item::Enum(it) => vec![it.ident.to_string()],
        syn::Item::Const(it) => vec![it.ident.to_string()],
        syn::Item::Static(it) => vec![it.ident.to_string()],
        syn::Item::Trait(it) => vec![it.ident.to_string()],
        syn::Item::Type(it) => vec![it.ident.to_string()],
        syn::Item::Use(it) => {
            let mut out = Vec::new();
            leaves(&it.tree, &mut out);
            out
        }
        _ => Vec::new(),
    }
}

fn leaves(tree: &syn::UseTree, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => leaves(&path.tree, out),
        syn::UseTree::Name(name) => out.push(name.ident.to_string()),
        syn::UseTree::Rename(rename) => {
            out.push(rename.ident.to_string());
            out.push(rename.rename.to_string());
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                leaves(item, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn ships(rel: &str, name: &str, marker: &str) -> Violation {
    Violation {
        gate: "artifact",
        location: rel.to_owned(),
        rule: "an item named for a test carries a cfg that keeps it out of the product build"
            .to_owned(),
        violation: format!("`{name}` says `{marker}` and nothing excludes it"),
        alternative: "put it behind `#[cfg(feature = \"conformance\")]`, declare that feature \
                      in the crate's manifest, and carry the same cfg on the re-export - \
                      `kernel::ledger` is the shape to copy"
            .to_owned(),
    }
}

fn product_features(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let path = root.join(PRODUCT);
    let text = walk::read_text(&path)?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: PRODUCT.to_owned(),
        msg: err.to_string(),
    })?;
    let Some(features) = parsed.get("features").and_then(toml::Value::as_table) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (name, enables) in features {
        let mut named: Vec<String> = vec![name.clone()];
        if let Some(list) = enables.as_array() {
            named.extend(
                list.iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned),
            );
        }
        for entry in named {
            if let Some(found) = TEST_FEATURES
                .iter()
                .find(|test| entry == **test || entry.ends_with(&format!("/{test}")))
            {
                out.push(Violation {
                    gate: "artifact",
                    location: PRODUCT.to_owned(),
                    rule: "the shipped binary enables no feature that exists to build test \
                           scaffolding"
                        .to_owned(),
                    violation: format!("feature `{name}` reaches `{found}` through `{entry}`"),
                    alternative: "enable it from the test that needs it - `--all-features` \
                                  is the build every suite compiles in, and the release \
                                  build is the one where none of them exists"
                        .to_owned(),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn found(source: &str) -> Vec<Violation> {
        let parsed = syn::parse_file(source).unwrap();
        let mut out = Vec::new();
        walk_items("crates/x/src/y.rs", &parsed.items, &mut out);
        out
    }

    #[test]
    fn an_unguarded_conformance_suite_is_refused() {
        let out = found("pub fn assert_port_conformance() {}\n");
        assert_eq!(out.len(), 1, "{out:#?}");
        assert!(out[0].violation.contains("conformance"));
    }

    #[test]
    fn the_same_suite_behind_a_feature_is_accepted() {
        assert!(found("#[cfg(feature = \"conformance\")]\npub mod conformance {}\n").is_empty());
    }

    #[test]
    fn a_re_export_is_a_declaration_for_this_purpose() {
        let out = found("pub use port::{BrowserPort, assert_port_conformance};\n");
        assert_eq!(out.len(), 1, "{out:#?}");
        assert!(
            found("#[cfg(feature = \"conformance\")]\npub use port::assert_port_conformance;\n")
                .is_empty()
        );
    }

    #[test]
    fn documenting_a_suite_is_not_shipping_one() {
        let source = "\
            /// Adapters are held to [`assert_port_conformance`].\n\
            pub trait BrowserPort {}\n";
        assert!(found(source).is_empty());
    }

    #[test]
    fn a_double_inside_a_test_module_is_left_alone() {
        let source = "#[cfg(test)]\nmod tests {\n    pub fn mock_clock() {}\n}\n";
        assert!(found(source).is_empty());
    }

    #[test]
    fn a_file_gated_for_tests_is_left_alone() {
        let source = "#![cfg(test)]\npub fn fake_clock() {}\n";
        let parsed = syn::parse_file(source).unwrap();
        let mut out = Vec::new();
        if !has_cfg(&parsed.attrs) {
            walk_items("Sprawling/x/src/y/tests.rs", &parsed.items, &mut out);
        }
        assert!(out.is_empty(), "{out:#?}");
    }
}
