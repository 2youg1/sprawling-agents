// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Dead dependencies: a key a manifest declares that no source file of
//! its package names, and a workspace dependency no package inherits
//! (xtask-SPEC.md section 8-37). rustc's `dead_code` already refuses a
//! dead private item under `-D warnings`; a dead manifest entry is the
//! kind of dead code no compiler pass reports.

use std::collections::BTreeSet;
use std::path::Path;

use crate::docnum::facts;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// The dependency tables a manifest may hold, at its top level and under
/// each `[target.*]` table alike.
const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

const RULE: &str = "every dependency a manifest declares is named by the code that compiles against it (xtask-SPEC.md section 8-37)";

/// Where a manifest lists the dependencies it declares only to constrain
/// resolution, which no source is expected to name.
const PINS: &str = "[package.metadata.unused] pins";

/// Every declared dependency no source of its package names, then every
/// workspace dependency no package inherits.
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let mut inherited = BTreeSet::new();
    for package in crate::members::members(root)?
        .into_iter()
        .map(|member| member.dir)
    {
        violations.extend(package_findings(root, &package, &mut inherited)?);
    }
    let manifest = facts::root_manifest(root)?;
    let workspace_keys = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(toml::Table::keys);
    for key in workspace_keys.filter(|key| !inherited.contains(*key)) {
        violations.push(Violation {
            gate: "unused",
            location: "Cargo.toml".to_owned(),
            rule: RULE.to_owned(),
            violation: format!(
                "`[workspace.dependencies]` declares `{key}`, and no package inherits it"
            ),
            alternative: format!("remove `{key}` from `[workspace.dependencies]`"),
        });
    }
    Ok(violations)
}

/// One package's findings: a declared key no source names unless its
/// manifest pins it, a pin some source names after all, and a pin no
/// dependency table declares. Every declared key is added to
/// `inherited`, which the workspace half of the gate reads.
fn package_findings(
    root: &Path,
    package: &str,
    inherited: &mut BTreeSet<String>,
) -> Result<Vec<Violation>, XtaskError> {
    let manifest = manifest(root, package)?;
    let declared = declared(&manifest);
    let pins = pins(&manifest, package)?;
    let sources = sources(&root.join(package))?;
    let finding = |violation: String, alternative: String| Violation {
        gate: "unused",
        location: format!("{package}/Cargo.toml"),
        rule: RULE.to_owned(),
        violation,
        alternative,
    };
    let mut violations = Vec::new();
    for (table, key) in &declared {
        let ident = key.replace('-', "_");
        match (names(&sources, &ident), pins.contains(key)) {
            (false, false) => violations.push(finding(
                format!(
                    "`[{table}]` declares `{key}`, and no `.rs` file under `{package}` names `{ident}`"
                ),
                format!("remove `{key}` from `{package}/Cargo.toml`"),
            )),
            (true, true) => violations.push(finding(
                format!(
                    "`{PINS}` lists `{key}`, and a `.rs` file under `{package}` names `{ident}`, so the exemption is no longer needed"
                ),
                format!("remove `{key}` from `pins`"),
            )),
            (true, false) | (false, true) => {}
        }
        inherited.insert(key.clone());
    }
    for pin in pins
        .iter()
        .filter(|pin| !declared.iter().any(|(_, key)| key == *pin))
    {
        violations.push(finding(
            format!("`{PINS}` lists `{pin}`, and no dependency table of `{package}` declares it"),
            format!("declare `{pin}`, or remove it from `pins`"),
        ));
    }
    Ok(violations)
}

/// The package's manifest, parsed.
fn manifest(root: &Path, package: &str) -> Result<toml::Table, XtaskError> {
    let file = format!("{package}/Cargo.toml");
    toml::from_str(&walk::read_text(&root.join(&file))?).map_err(|err| XtaskError::Doc {
        file,
        msg: format!("the manifest does not parse: {err}"),
    })
}

/// `(table, key)` for every dependency the manifest declares, in
/// manifest order, `[target.*]` tables after the top-level ones.
fn declared(manifest: &toml::Table) -> Vec<(&'static str, String)> {
    let targets = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(toml::Table::values)
        .filter_map(toml::Value::as_table);
    std::iter::once(manifest)
        .chain(targets)
        .flat_map(|scope| {
            TABLES.into_iter().flat_map(move |table| {
                scope
                    .get(table)
                    .and_then(toml::Value::as_table)
                    .into_iter()
                    .flat_map(move |deps| deps.keys().map(move |key| (table, key.clone())))
            })
        })
        .collect()
}

/// The keys the manifest pins, in the order it lists them; none when the
/// manifest has no `pins`.
///
/// # Errors
/// Refuses a `pins` that is not an array of strings, because a pin the
/// gate cannot read is an exemption nobody can check.
fn pins(manifest: &toml::Table, package: &str) -> Result<Vec<String>, XtaskError> {
    let Some(listed) = manifest
        .get("package")
        .and_then(|table| table.get("metadata"))
        .and_then(|table| table.get("unused"))
        .and_then(|table| table.get("pins"))
    else {
        return Ok(Vec::new());
    };
    let unreadable = || XtaskError::Doc {
        file: format!("{package}/Cargo.toml"),
        msg: format!("`{PINS}` is not an array of dependency keys"),
    };
    listed
        .as_array()
        .ok_or_else(unreadable)?
        .iter()
        .map(|pin| pin.as_str().map(str::to_owned).ok_or_else(unreadable))
        .collect()
}

/// Every `.rs` file under the package directory, joined into one text.
fn sources(dir: &Path) -> Result<String, XtaskError> {
    walk::files_with_ext(dir, &["rs"])?
        .iter()
        .map(|path| walk::read_text(path).map(|text| text + "\n"))
        .collect()
}

/// Whether `ident` occurs in `text` as a whole identifier rather than as
/// part of a longer one.
fn names(text: &str, ident: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(ident).any(|(at, _)| {
        let before = text.get(..at).and_then(|head| head.chars().next_back());
        let after = at
            .checked_add(ident.len())
            .and_then(|end| text.get(end..))
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn names_a_dependency_no_source_names_and_a_workspace_entry_no_package_inherits() {
        let root = std::env::temp_dir().join(format!("xtask-unused-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.dependencies]\nused = \"1\"\norphan = \"1\"\n",
        );
        write(
            &root,
            "a/Cargo.toml",
            "[package]\nname = \"a\"\n\n[dependencies]\nused = { workspace = true }\nsplit-name = \"1\"\nnamed_prefix = \"1\"\n\n[dev-dependencies]\nghost = \"1\"\n\n[target.'cfg(windows)'.dependencies]\nwinonly = \"1\"\n",
        );
        write(
            &root,
            "a/src/lib.rs",
            "use used::Thing;\nfn f() { split_name::go(); named_prefix_more::go(); }\n",
        );
        write(&root, "a/tests/t.rs", "#[test] fn t() { winonly::go(); }\n");

        let found: Vec<(String, String)> = check(&root)
            .unwrap()
            .into_iter()
            .map(|v| (v.location, v.violation))
            .collect();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(
            found,
            vec![
                (
                    "a/Cargo.toml".to_owned(),
                    "`[dependencies]` declares `named_prefix`, and no `.rs` file under `a` names `named_prefix`".to_owned()
                ),
                (
                    "a/Cargo.toml".to_owned(),
                    "`[dev-dependencies]` declares `ghost`, and no `.rs` file under `a` names `ghost`".to_owned()
                ),
                (
                    "Cargo.toml".to_owned(),
                    "`[workspace.dependencies]` declares `orphan`, and no package inherits it".to_owned()
                ),
            ]
        );
    }

    /// A dependency declared only to constrain resolution is exempt from
    /// naming when its manifest lists it under `pins`, and a pin that
    /// points at nothing or is no longer needed is itself a finding
    /// (xtask-SPEC.md section 8-37).
    #[test]
    fn a_dependency_declared_to_pin_a_resolution_is_listed_in_its_manifest() {
        let root = std::env::temp_dir().join(format!("xtask-unused-pins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(&root, "Cargo.toml", "[workspace]\nmembers = [\"a\"]\n");
        write(
            &root,
            "a/Cargo.toml",
            "[package]\nname = \"a\"\n\n[dependencies]\npinned = \"1\"\nnamed = \"1\"\n\n[package.metadata.unused]\npins = [\"pinned\", \"ghost\", \"named\"]\n",
        );
        write(&root, "a/src/lib.rs", "use named as _;\n");

        let found: Vec<(String, String)> = check(&root)
            .unwrap()
            .into_iter()
            .map(|v| (v.location, v.violation))
            .collect();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(
            found,
            vec![
                (
                    "a/Cargo.toml".to_owned(),
                    "`[package.metadata.unused] pins` lists `named`, and a `.rs` file under `a` names `named`, so the exemption is no longer needed".to_owned()
                ),
                (
                    "a/Cargo.toml".to_owned(),
                    "`[package.metadata.unused] pins` lists `ghost`, and no dependency table of `a` declares it".to_owned()
                ),
            ]
        );
    }
}
