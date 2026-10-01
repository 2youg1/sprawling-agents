// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wall around `desktop/`, which is a copy of the workspace's own
//! and must stay one.
//!
//! `desktop` and its FFI seam `desktop/ffi` are the packages this
//! repository builds from outside the workspace (root `Cargo.toml`,
//! `exclude`), and they sit there for one recorded reason: each call
//! into the Zig leaf relaxes `unsafe_code`, which `forbid` makes
//! impossible and `deny` makes visible (desktop-SPEC.md section 8.5,
//! second pair). `desktop/Cargo.toml` is the root of a workspace of
//! their own, and everything else about it is a **copy** of the root
//! workspace's — the lint tables and the package metadata, stated once
//! in `[workspace.lints]` and `[workspace.package]`, and the version of
//! every dependency both sides name — and a copy is a second home for a
//! fact. Every package inside the wall inherits that copy, which is
//! checked too: a package that wrote a table of its own would stand
//! outside the comparison (xtask-SPEC.md section 8-46).
//!
//! **This is the shape `guard`'s own rule cannot see.** The trailer
//! rule catches a gate loosened in the same commit as the work it would
//! have refused. A copied lint table needs no such commit: one side is
//! edited, the other is not, and nothing red follows. So the copy is
//! compared here, key by key, and a difference is a refusal unless it
//! is one of the differences this module records with its reason.
//!
//! **A recorded difference cleans itself.** A row whose two sides have
//! become equal is struck, exactly as `length` strikes a register entry
//! for a file that came back under budget: an exception nobody needs is
//! an exception nobody decided to keep granting.
//!
//! The facts the package quotes from source files inside the wall — the
//! protocol revision, the effect-unknown key, the error codes and the
//! quality domain — are compared for the same reason, in `quoted`.

use std::path::Path;

use crate::report::{Violation, XtaskError};

mod quoted;

const ROOT_MANIFEST: &str = "Cargo.toml";
const DESKTOP_MANIFEST: &str = "desktop/Cargo.toml";
/// The package metadata every workspace member inherits from
/// `[workspace.package]` and this package restates by hand.
const SHARED_METADATA: [&str; 5] = ["version", "edition", "license", "rust-version", "publish"];

/// One key the two walls are allowed to disagree about, and why.
///
/// The reason is printed when the disagreement disappears, so whoever
/// strikes the row reads what it was for.
struct Recorded {
    table: &'static str,
    key: &'static str,
    because: &'static str,
}

/// Every difference between the two walls that somebody decided.
///
/// Nothing else may differ. Adding a row here is a re-pricing of the
/// rule and lands in `tools/xtask/`, which `guard`'s trailer rule watches.
const RECORDED: [Recorded; 2] = [
    Recorded {
        table: "rust",
        key: "unsafe_code",
        because: "each call into the Zig leaf that carries the Win32 calls relaxes it at that one \
                  statement with a written reason, which `forbid` makes impossible (desktop-SPEC.md \
                  section 8.5, second pair, and section 8-12)",
    },
    Recorded {
        table: "rust",
        key: "unexpected_cfgs",
        because: "the workspace declares `cfg(kani)` because kani runs against its crates; this \
                  package has no harness, so declaring that name here would be a second home \
                  for a fact that is not true of it",
    },
];

/// The one member whose lint table is its own.
#[cfg(test)]
const LEAF: &str = "crates/desktop/ffi";

/// Every member's lint table, judged against the workspace's.
#[cfg(test)]
fn tables(_workspace: &toml::Value, _members: &[(String, toml::Value)]) -> Vec<Violation> {
    Vec::new()
}

pub(super) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let workspace = manifest(root, ROOT_MANIFEST)?;
    let desktop = manifest(root, DESKTOP_MANIFEST)?;
    let mut violations = Vec::new();
    metadata(&workspace, &desktop, &mut violations);
    lints(&workspace, &desktop, &mut violations);
    dependencies(&workspace, &desktop, &mut violations);
    inherited(&packages(root, &desktop)?, &mut violations);
    quoted::check(root, &mut violations)?;
    Ok(violations)
}

/// Every package inside the wall: the root package of
/// `desktop/Cargo.toml`, then each of its `[workspace] members`, each
/// with the manifest path a refusal names.
fn packages(root: &Path, desktop: &toml::Value) -> Result<Vec<(String, toml::Value)>, XtaskError> {
    let members = desktop
        .get("workspace")
        .and_then(|it| it.get("members"))
        .and_then(toml::Value::as_array)
        .map(|listed| {
            listed
                .iter()
                .filter_map(toml::Value::as_str)
                .map(|member| format!("desktop/{member}/Cargo.toml"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut found = vec![(DESKTOP_MANIFEST.to_owned(), desktop.clone())];
    for rel in members {
        let read = manifest(root, &rel)?;
        found.push((rel, read));
    }
    Ok(found)
}

/// The package metadata, which every member inherits and the wall
/// restates once.
fn metadata(workspace: &toml::Value, desktop: &toml::Value, out: &mut Vec<Violation>) {
    let inherited = workspace.get("workspace").and_then(|it| it.get("package"));
    let restated = desktop.get("workspace").and_then(|it| it.get("package"));
    for key in SHARED_METADATA {
        let expected = inherited.and_then(|table| table.get(key));
        let found = restated.and_then(|table| table.get(key));
        if expected == found {
            continue;
        }
        out.push(diverged(
            format!("{DESKTOP_MANIFEST} [workspace.package] {key}"),
            "the out-of-tree workspace states the metadata every member inherits from \
             `[workspace.package]`, and states the same value",
            format!(
                "{} against the workspace's {}",
                shown(found),
                shown(expected)
            ),
            format!("set `{key}` in {DESKTOP_MANIFEST} to what `[workspace.package]` says"),
        ));
    }
}

/// The two lint tables, key by key, in both directions.
fn lints(workspace: &toml::Value, desktop: &toml::Value, out: &mut Vec<Violation>) {
    for table in ["rust", "clippy"] {
        let wall = workspace
            .get("workspace")
            .and_then(|it| it.get("lints"))
            .and_then(|it| it.get(table));
        let copy = desktop
            .get("workspace")
            .and_then(|it| it.get("lints"))
            .and_then(|it| it.get(table));
        let mut named = keys(wall);
        named.extend(keys(copy));
        for key in named {
            judge(
                Compared {
                    table,
                    key: &key,
                    wall: wall.and_then(|it| it.get(&key)),
                    copy: copy.and_then(|it| it.get(&key)),
                },
                out,
            );
        }
    }
}

/// One lint key as the two walls spell it.
struct Compared<'a> {
    table: &'a str,
    key: &'a str,
    wall: Option<&'a toml::Value>,
    copy: Option<&'a toml::Value>,
}

/// One lint key, judged against the record of decided differences.
fn judge(compared: Compared<'_>, out: &mut Vec<Violation>) {
    let Compared {
        table,
        key,
        wall,
        copy,
    } = compared;
    let recorded = RECORDED
        .iter()
        .find(|row| row.table == table && row.key == key);
    let location = format!("{DESKTOP_MANIFEST} [workspace.lints.{table}] {key}");
    match (recorded, wall == copy) {
        // A decided difference that has become no difference is struck,
        // the way a register entry is struck when its file comes back
        // under budget.
        (Some(row), true) => out.push(diverged(
            format!("tools/xtask/src/guard/wall.rs RECORDED {table}.{key}"),
            "a recorded difference between the two walls is struck once the two sides agree",
            format!("`{key}` now reads the same on both sides"),
            format!(
                "delete its row: it was granted because {}, and that reason no longer shows",
                row.because
            ),
        )),
        (None, false) => out.push(diverged(
            location,
            "the lint wall of the out-of-tree package is the workspace's own, key for key",
            format!("{} against the workspace's {}", shown(copy), shown(wall)),
            format!(
                "copy the workspace's line into {DESKTOP_MANIFEST}, or record the difference \
                 and its reason in `RECORDED` (tools/xtask/src/guard/wall.rs) — a difference nobody \
                 wrote down is a wall that fell over quietly"
            ),
        )),
        (Some(_), false) | (None, true) => {}
    }
}

/// Every package inside the wall inherits it: `[lints]` is exactly
/// `workspace = true`, and each shared metadata key is
/// `{ workspace = true }`. A package that states either itself is a
/// copy the comparison above does not read.
fn inherited(packages: &[(String, toml::Value)], out: &mut Vec<Violation>) {
    for (rel, package) in packages {
        if !inheriting(package.get("lints")) {
            out.push(diverged(
                format!("{rel} [lints]"),
                "every package inside the wall inherits its lint table",
                format!("{} against `workspace = true`", shown(package.get("lints"))),
                format!("write `[lints]` in {rel} as `workspace = true` and nothing else"),
            ));
        }
        for key in SHARED_METADATA {
            let found = package.get("package").and_then(|it| it.get(key));
            if inheriting(found) {
                continue;
            }
            out.push(diverged(
                format!("{rel} [package] {key}"),
                "every package inside the wall inherits its package metadata",
                format!("{} against `{{ workspace = true }}`", shown(found)),
                format!("write `{key}.workspace = true` in {rel}"),
            ));
        }
    }
}

/// Whether a value is a table holding `workspace = true` and nothing
/// else.
fn inheriting(value: Option<&toml::Value>) -> bool {
    value.and_then(toml::Value::as_table).is_some_and(|table| {
        table.len() == 1 && table.get("workspace") == Some(&toml::Value::Boolean(true))
    })
}

/// The version of every dependency both manifests name.
///
/// Features are left alone: this package switches on what its six tools
/// reach and nothing else, which is a statement about it rather than a
/// copy of anything. A version line is the copy.
fn dependencies(workspace: &toml::Value, desktop: &toml::Value, out: &mut Vec<Violation>) {
    let shared = workspace
        .get("workspace")
        .and_then(|it| it.get("dependencies"));
    for section in ["dependencies", "dev-dependencies"] {
        let Some(named) = desktop.get(section).and_then(toml::Value::as_table) else {
            continue;
        };
        for (name, held) in named {
            let Some(theirs) = shared.and_then(|it| it.get(name)).and_then(required) else {
                continue;
            };
            let ours = required(held);
            if ours == Some(theirs) {
                continue;
            }
            out.push(diverged(
                format!("{DESKTOP_MANIFEST} [{section}] {name}"),
                "a dependency both manifests name is held to one version line",
                format!(
                    "`{name}` at {} against the workspace's `{theirs}`",
                    ours.map_or_else(|| "no version".to_owned(), |it| format!("`{it}`")),
                ),
                format!(
                    "set `{name}` to `{theirs}`, the version `[workspace.dependencies]` states"
                ),
            ));
        }
    }
}

/// The version requirement of one dependency, however it is written.
fn required(held: &toml::Value) -> Option<&str> {
    match held {
        toml::Value::String(line) => Some(line),
        other => other.get("version").and_then(toml::Value::as_str),
    }
}

/// One manifest, read as a value.
fn manifest(root: &Path, rel: &str) -> Result<toml::Value, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: rel.to_owned(),
        msg: format!("this manifest does not parse as TOML: {err}"),
    })
}

/// The keys of one table. A table that is absent has no keys, which is
/// the reading that lets a key present on one side only be compared
/// against nothing on the other.
fn keys(table: Option<&toml::Value>) -> std::collections::BTreeSet<String> {
    match table.and_then(toml::Value::as_table) {
        Some(found) => found.keys().cloned().collect(),
        None => std::collections::BTreeSet::new(),
    }
}

/// One value as a refusal prints it, including its absence.
///
/// A lint is as often a table (`{ level = "deny", … }`) as a bare
/// string, and the table is shown as Debug rather than re-serialised:
/// what a reader needs is the difference, not valid TOML.
fn shown(value: Option<&toml::Value>) -> String {
    match value {
        None => "absent".to_owned(),
        Some(toml::Value::String(line)) => format!("`{line}`"),
        Some(table) => format!("`{table:?}`"),
    }
}

fn diverged(
    location: String,
    rule: &'static str,
    violation: String,
    alternative: String,
) -> Violation {
    Violation {
        gate: "guard",
        location,
        rule: rule.to_owned(),
        violation,
        alternative,
    }
}

#[cfg(test)]
mod tests;
