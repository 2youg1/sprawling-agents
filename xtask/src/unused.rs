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

/// Every declared dependency no source of its package names, then every
/// workspace dependency no package inherits.
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let mut inherited = BTreeSet::new();
    for package in crate::members::members(root)?
        .into_iter()
        .map(|member| member.dir)
    {
        let declared = declared(root, &package)?;
        let sources = sources(&root.join(&package))?;
        for (table, key) in declared {
            let ident = key.replace('-', "_");
            if !names(&sources, &ident) {
                violations.push(Violation {
                    gate: "unused",
                    location: format!("{package}/Cargo.toml"),
                    rule: RULE.to_owned(),
                    violation: format!(
                        "`[{table}]` declares `{key}`, and no `.rs` file under `{package}` names `{ident}`"
                    ),
                    alternative: format!("remove `{key}` from `{package}/Cargo.toml`"),
                });
            }
            inherited.insert(key);
        }
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

/// `(table, key)` for every dependency the package's manifest declares,
/// in manifest order, `[target.*]` tables after the top-level ones.
fn declared(root: &Path, package: &str) -> Result<Vec<(&'static str, String)>, XtaskError> {
    let file = format!("{package}/Cargo.toml");
    let manifest: toml::Table =
        toml::from_str(&walk::read_text(&root.join(&file))?).map_err(|err| XtaskError::Doc {
            file: file.clone(),
            msg: format!("the manifest does not parse: {err}"),
        })?;
    let targets = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(toml::Table::values)
        .filter_map(toml::Value::as_table);
    Ok(std::iter::once(&manifest)
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
        .collect())
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
}
