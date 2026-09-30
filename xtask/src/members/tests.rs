// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// The roster this tree gives: the kernel where its directory is, the
/// package outside the lint wall reached through its dependent, the
/// gates' own package declared a tool, and no package at the root, which
/// holds the workspace table rather than a package.
#[test]
fn this_checkout_lists_its_members_and_the_package_outside_the_wall() {
    let found = members(crate::root::this_checkout()).unwrap();
    let kernel = found.iter().find(|m| m.dir == "crates/kernel").unwrap();
    assert_eq!(kernel.lib.as_deref(), Some("kernel"));
    assert_eq!(
        (kernel.role, kernel.reach),
        (Role::Product, Reach::Workspace)
    );
    let outside: Vec<&str> = found
        .iter()
        .filter(|m| m.reach == Reach::PathDependency)
        .map(|m| m.dir.as_str())
        .collect();
    assert_eq!(outside, ["desktop"]);
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

/// What `check-branch` selects tests by: the workspace packages holding a
/// changed path, each once. Documents, Lean models, deleted paths and the
/// package outside the workspace select nothing.
#[test]
fn owning_names_each_workspace_package_once_and_skips_the_rest() {
    let member = |package: &str, dir: &str, reach| Member {
        package: package.to_owned(),
        lib: None,
        dir: dir.to_owned(),
        role: Role::Product,
        reach,
        depends_on: BTreeSet::new(),
    };
    let found = [
        member("sprawling-j", "crates/j", Reach::Workspace),
        member("sprawling-k", "tools/k", Reach::Workspace),
        member("sprawling-desktop", "desktop", Reach::PathDependency),
    ];
    let paths = [
        "crates/j/src/lib.rs",
        "crates/j/src/a.rs\r",
        "crates/j/j-SPEC.md",
        "tools/k/Spec.lean",
        "crates/jx/src/lib.rs",
        "gone/src/lib.rs",
        "desktop/src/lib.rs",
    ]
    .map(str::to_owned);
    assert_eq!(
        owning(&found, &paths).into_iter().collect::<Vec<_>>(),
        ["sprawling-j"]
    );
}
