// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The readers behind [`super::FACTS`]: each one opens the code that
//! decides a fact and renders it the way a document writes it.
//!
//! Nothing here holds a value. A reader that cannot find its subject
//! refuses with the file it looked in, because a fact that quietly
//! recounts to zero would rewrite a document into a lie.

use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

use super::{packages, register_row, root_manifest};

/// The wire tags of one frame family, as a document writes them:
/// `` `dispatch`, `wake`, … `` in the order the schema declares.
///
/// The tags are read out of the schema rather than lower-cased from the
/// variant names, because `rename_all = "snake_case"` is `channels`'
/// decision and a second implementation of it here would be a second
/// authority for how a frame is spelled on the wire.
pub(super) fn wire_tags(kind: &str) -> Result<String, XtaskError> {
    let schema = channels::wire_schema();
    let members = schema
        .get("$defs")
        .and_then(|defs| defs.get(kind))
        .and_then(|family| family.get("oneOf"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| XtaskError::Doc {
            file: "channels::wire_schema".to_owned(),
            msg: format!("`$defs.{kind}` is not a `oneOf` of variants"),
        })?;
    let mut tags = Vec::with_capacity(members.len());
    for member in members {
        // A variant with a payload is an object whose one property is
        // the tag; the variants that carry nothing arrive together as
        // one member listing their tags as a string enum.
        if let Some(properties) = member
            .get("properties")
            .and_then(serde_json::Value::as_object)
        {
            tags.extend(properties.keys().map(|tag| format!("`{tag}`")));
            continue;
        }
        if let Some(only) = member.get("const").and_then(serde_json::Value::as_str) {
            tags.push(format!("`{only}`"));
            continue;
        }
        let listed = member
            .get("enum")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| XtaskError::Doc {
                file: "channels::wire_schema".to_owned(),
                msg: format!("a `{kind}` variant is neither an object nor a listed tag"),
            })?;
        for tag in listed {
            let Some(tag) = tag.as_str() else {
                return Err(XtaskError::Doc {
                    file: "channels::wire_schema".to_owned(),
                    msg: format!("a `{kind}` tag is not a string"),
                });
            };
            tags.push(format!("`{tag}`"));
        }
    }
    if tags.is_empty() {
        return Err(XtaskError::Doc {
            file: "channels::wire_schema".to_owned(),
            msg: format!("`$defs.{kind}` declares no variant"),
        });
    }
    Ok(tags.join(", "))
}

/// How many packages the lockfile resolves, workspace members included,
/// which is the number `sprawling status --deps` lists.
pub(super) fn dependency_count(root: &Path) -> Result<String, XtaskError> {
    let text = walk::read_text(&root.join("Cargo.lock"))?;
    let found = text.lines().filter(|line| *line == "[[package]]").count();
    if found == 0 {
        return Err(XtaskError::Doc {
            file: "Cargo.lock".to_owned(),
            msg: "no `[[package]]` entries; the lockfile format changed".to_owned(),
        });
    }
    Ok(found.to_string())
}

/// How many Rust files sit in one directory of every package: the shape
/// `crates/*/tests/ui` has, where each file is one counterexample.
pub(super) fn files_under(root: &Path, inside: &str, what: &str) -> Result<String, XtaskError> {
    let mut found = 0_usize;
    for package in packages(root)? {
        let base = root.join(&package).join(inside);
        if !base.is_dir() {
            continue;
        }
        found = found.saturating_add(walk::files_with_ext(&base, &["rs"])?.len());
    }
    refuse_if_empty(found, inside, what)
}

/// How many Rust files sit in one directory of the tree.
pub(super) fn in_directory(root: &Path, dir: &str, what: &str) -> Result<String, XtaskError> {
    let base = root.join(dir);
    if !base.is_dir() {
        return Err(XtaskError::Doc {
            file: dir.to_owned(),
            msg: format!("no such directory, so no {what} can be counted"),
        });
    }
    refuse_if_empty(walk::files_with_ext(&base, &["rs"])?.len(), dir, what)
}

/// How many test functions the tree declares, across every package.
///
/// This counts what a person means by "a test": one `#[test]` or
/// `#[tokio::test]` function. A property test is one of them and runs a
/// case list of its own, so the reading is a floor rather than a number
/// of assertions, and the document that quotes it says so.
pub(super) fn test_functions(root: &Path) -> Result<String, XtaskError> {
    let mut found = 0_usize;
    for package in packages(root)? {
        let base = root.join(&package);
        if !base.is_dir() {
            continue;
        }
        for file in walk::files_with_ext(&base, &["rs"])? {
            if walk::in_isolation_zone(&walk::rel(root, &file)) {
                continue;
            }
            let text = walk::read_text(&file)?;
            found = found.saturating_add(
                text.lines()
                    .filter(|line| {
                        let trimmed = line.trim();
                        trimmed == "#[test]" || trimmed == "#[tokio::test]"
                    })
                    .count(),
            );
        }
    }
    refuse_if_empty(found, "the tree", "test function")
}

/// The version this workspace pins for one crate.
///
/// Every manifest is read, and two manifests pinning one crate at two
/// versions is itself the refusal: the document cannot quote a version
/// that the tree does not agree on.
pub(super) fn dep_version(root: &Path, crate_name: &str) -> Result<String, XtaskError> {
    let mut found: Vec<(String, String)> = Vec::new();
    if let Some(pinned) = pinned_in(&root_manifest(root)?, crate_name) {
        found.push(("Cargo.toml".to_owned(), pinned));
    }
    for package in packages(root)? {
        let manifest = root.join(&package).join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let rel = walk::rel(root, &manifest);
        let text = walk::read_text(&manifest)?;
        let parsed: toml::Value = toml::from_str(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this manifest does not parse: {err}"),
        })?;
        if let Some(pinned) = pinned_in(&parsed, crate_name) {
            found.push((rel, pinned));
        }
    }
    let Some((_, first)) = found.first() else {
        return Err(XtaskError::Doc {
            file: "Cargo.toml".to_owned(),
            msg: format!("no manifest in this workspace pins `{crate_name}`"),
        });
    };
    if let Some((where_else, other)) = found.iter().find(|(_, version)| version != first) {
        return Err(XtaskError::Doc {
            file: where_else.clone(),
            msg: format!("`{crate_name}` is pinned at {other} here and at {first} elsewhere"),
        });
    }
    Ok(first.clone())
}

/// The version one manifest states for a crate, wherever a manifest may
/// state it: the workspace table, this package's own table, and the
/// per-target tables a platform arm uses.
fn pinned_in(manifest: &toml::Value, crate_name: &str) -> Option<String> {
    let direct = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|table| version_of(table, crate_name))
        .or_else(|| {
            manifest
                .get("dependencies")
                .and_then(|table| version_of(table, crate_name))
        });
    if direct.is_some() {
        return direct;
    }
    let targets = manifest.get("target")?.as_table()?;
    targets.values().find_map(|arm| {
        arm.get("dependencies")
            .and_then(|table| version_of(table, crate_name))
    })
}

/// The version a dependency table states, in either spelling: a bare
/// string, or a table with a `version` key.
fn version_of(table: &toml::Value, crate_name: &str) -> Option<String> {
    let entry = table.get(crate_name)?;
    match entry {
        toml::Value::String(version) => Some(version.clone()),
        other => other.get("version")?.as_str().map(str::to_owned),
    }
}

/// One byte count of a register row, grouped the way a document writes
/// it: `288,972 B`.
pub(super) fn grouped_bytes(root: &Path, row: &str, field: &str) -> Result<String, XtaskError> {
    Ok(format!("{} B", grouped(bytes_of(root, row, field)?)))
}

/// How many times the budget of a register row exceeds its reading, to
/// one decimal: `7.3×`.
pub(super) fn headroom(root: &Path, row: &str) -> Result<String, XtaskError> {
    let budget = bytes_of(root, row, "budget_bytes")?;
    let reading = bytes_of(root, row, "best_bytes")?;
    // Rounded to the nearest tenth rather than truncated: a document
    // that states 7.2 where the ratio is 7.257 is the same kind of
    // wrong this gate exists to stop.
    let tenths = budget
        .checked_mul(10)
        .and_then(|scaled| scaled.checked_add(reading.checked_div(2)?))
        .and_then(|rounded| rounded.checked_div(reading))
        .ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("[{row}] has no headroom to state: its reading is zero"),
        })?;
    Ok(format!(
        "{}.{}\u{d7}",
        tenths.saturating_div(10),
        tenths % 10
    ))
}

/// One unsigned field of a register row.
fn bytes_of(root: &Path, row: &str, field: &str) -> Result<u64, XtaskError> {
    let stated = register_row(root, row)?
        .get(field)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("[{row}] states no `{field}`"),
        })?;
    u64::try_from(stated).map_err(|_| XtaskError::Doc {
        file: "xtask/budgets.toml".to_owned(),
        msg: format!("[{row}].{field} is not a byte count: {stated}"),
    })
}

/// A byte count with a comma every three digits, which is how every
/// size in this repository's documents is written.
fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len().saturating_add(digits.len() / 3));
    for (index, digit) in digits.chars().enumerate() {
        let left = digits.len().saturating_sub(index);
        if index != 0 && left % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A count of zero means the reader looked in the wrong place, so it is
/// a refusal rather than a number a document would be rewritten to.
fn refuse_if_empty(found: usize, where_looked: &str, what: &str) -> Result<String, XtaskError> {
    if found == 0 {
        return Err(XtaskError::Doc {
            file: where_looked.to_owned(),
            msg: format!("no {what} found here; this reader is looking in the wrong place"),
        });
    }
    Ok(found.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_byte_count_is_grouped_every_three_digits() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(288_972), "288,972");
        assert_eq!(grouped(2_097_152), "2,097,152");
    }

    #[test]
    fn a_dependency_table_states_a_version_in_either_spelling() {
        let manifest: toml::Value = toml::from_str(
            "[dependencies]\n\
             toml = \"1.1\"\n\
             axum = { version = \"0.8\", features = [\"ws\"] }\n",
        )
        .unwrap();
        assert_eq!(pinned_in(&manifest, "toml").as_deref(), Some("1.1"));
        assert_eq!(pinned_in(&manifest, "axum").as_deref(), Some("0.8"));
        assert_eq!(pinned_in(&manifest, "absent"), None);
    }

    #[test]
    fn a_platform_arm_pins_as_well_as_the_package_does() {
        let manifest: toml::Value = toml::from_str(
            "[target.'cfg(windows)'.dependencies]\nwindows = { version = \"0.62\" }\n",
        )
        .unwrap();
        assert_eq!(pinned_in(&manifest, "windows").as_deref(), Some("0.62"));
    }

    #[test]
    fn the_wire_tags_are_the_snake_case_keys_the_schema_declares() {
        let commands = wire_tags("Command").unwrap();
        assert!(commands.contains("`dispatch`"), "{commands}");
        assert!(commands.contains("`put_document`"), "{commands}");
        assert_eq!(
            commands.matches('`').count() / 2,
            channels::COMMAND_NAMES.len()
        );
        let queries = wire_tags("Query").unwrap();
        assert_eq!(
            queries.matches('`').count() / 2,
            channels::QUERY_NAMES.len()
        );
    }
}
