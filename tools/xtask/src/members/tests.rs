// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// The roster this tree gives: the kernel where its directory is, the
/// desktop server and the seam nested in its directory each as a package
/// of its own, the gates' own package declared a tool, and no package at
/// the root, which holds the workspace table rather than a package.
#[test]
fn this_checkout_lists_its_members_and_the_package_nested_in_one() {
    let found = members(crate::root::this_checkout()).unwrap();
    let kernel = found.iter().find(|m| m.dir == "crates/kernel").unwrap();
    assert_eq!(kernel.lib.as_deref(), Some("kernel"));
    assert_eq!(kernel.role, Role::Product);
    let desktop: Vec<(&str, Option<&str>)> = found
        .iter()
        .filter(|m| m.dir.starts_with("crates/desktop"))
        .map(|m| (m.dir.as_str(), m.lib.as_deref()))
        .collect();
    assert_eq!(
        desktop,
        [
            ("crates/desktop", Some("desktop")),
            ("crates/desktop/ffi", Some("desktop_ffi"))
        ]
    );
    let gates = found.iter().find(|m| m.package == "xtask").unwrap();
    assert_eq!((gates.role, gates.lib.as_deref()), (Role::Tool, None));
    assert!(found.iter().all(|m| !m.dir.is_empty()), "{found:?}");
}

/// A package whose manifest sits outside the checkout is refused: kept
/// absolute, it would be read from another tree.
#[test]
fn a_package_outside_the_checkout_is_refused() {
    let metadata = serde_json::json!({
        "workspace_root": "/checkout",
        "packages": [{
            "name": "elsewhere",
            "manifest_path": "/other/elsewhere/Cargo.toml",
            "targets": [],
            "dependencies": [],
            "metadata": null,
        }],
    });
    let verdict = read(&metadata);
    assert!(
        matches!(&verdict, Err(XtaskError::OutsideCheckout { package, .. }) if package == "elsewhere"),
        "{verdict:?}"
    );
}

/// One package, as a fixture names it.
fn member(package: &str, dir: &str) -> Member {
    Member {
        package: package.to_owned(),
        lib: None,
        dir: dir.to_owned(),
        role: Role::Product,
        depends_on: BTreeSet::new(),
    }
}

/// What `check-branch` selects tests by: the workspace packages holding a
/// changed path, each once. Documents, Lean models and deleted paths
/// select nothing.
#[test]
fn owning_names_each_workspace_package_once_and_skips_the_rest() {
    let found = [
        member("sprawling-j", "crates/j"),
        member("sprawling-k", "tools/k"),
    ];
    let paths = [
        "crates/j/src/lib.rs",
        "crates/j/src/a.rs\r",
        "crates/j/j-SPEC.md",
        "tools/k/Spec.lean",
        "crates/jx/src/lib.rs",
        "gone/src/lib.rs",
    ]
    .map(str::to_owned);
    assert_eq!(
        owning(&found, &paths).into_iter().collect::<Vec<_>>(),
        ["sprawling-j"]
    );
}

/// A path inside a package nested in another belongs to the nested one:
/// `crates/desktop/ffi` is a package of its own, though the directory of
/// `crates/desktop` holds it too, so a change to the seam selects the
/// seam's tests and a change beside it selects the server's.
#[test]
fn a_path_in_a_nested_package_belongs_to_the_nested_package() {
    let found = [
        member("sprawling-desktop", "crates/desktop"),
        member("sprawling-desktop-ffi", "crates/desktop/ffi"),
    ];
    let seam = ["crates/desktop/ffi/src/leaf.rs".to_owned()];
    assert_eq!(
        owning(&found, &seam).into_iter().collect::<Vec<_>>(),
        ["sprawling-desktop-ffi"]
    );
    let server = ["crates/desktop/src/lib.rs".to_owned()];
    assert_eq!(
        owning(&found, &server).into_iter().collect::<Vec<_>>(),
        ["sprawling-desktop"]
    );
}
