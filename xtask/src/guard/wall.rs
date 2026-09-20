// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wall around `desktop/`, which is a copy of the workspace's own
//! and must stay one.
//!
//! `desktop` is the one package this repository builds from outside the
//! workspace (root `Cargo.toml`, `exclude`), and it sits there for one
//! recorded reason: the Win32 boundary relaxes `unsafe_code` at a few
//! call sites, which `forbid` makes impossible and `deny` makes visible
//! (desktop-SPEC.md section 8.5, second pair). Everything else about
//! that package's manifest is a **copy** of the workspace's — the lint
//! tables, the package metadata, the version of every dependency both
//! sides name — and a copy is a second home for a fact.
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
//! Two facts beyond the manifest are compared for the same reason. The
//! protocol revision this server speaks is `protocol::PROTOCOL_VERSION`
//! written a second time, because the two ends must agree to talk at
//! all; and every `E_` code the package spells is required to be one
//! `kernel` already defines, which is the boundary section 8.5 drew
//! around that duplication — the package may quote the city's spelling
//! and may never mint a code of its own.

use std::path::Path;

use crate::report::{Violation, XtaskError};

const ROOT_MANIFEST: &str = "Cargo.toml";
const DESKTOP_MANIFEST: &str = "desktop/Cargo.toml";
/// Where the protocol revision is decided, and where it is copied.
const PROTOCOL_HOME: &str = "crates/protocol/src/mcp/handshake.rs";
const PROTOCOL_COPY: &str = "desktop/src/rpc.rs";
/// Where the city's error codes are defined, and where the out-of-tree
/// package quotes them.
const CODE_HOME: &str = "crates/kernel/src/error/code.rs";
const CODE_QUOTE: &str = "desktop/src/refusal.rs";

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
/// rule and lands in `xtask/`, which `guard`'s trailer rule watches.
const RECORDED: [Recorded; 2] = [
    Recorded {
        table: "rust",
        key: "unsafe_code",
        because: "the Win32 boundary relaxes it at one call site with a written reason, which \
                  `forbid` makes impossible (desktop-SPEC.md section 8.5, second pair)",
    },
    Recorded {
        table: "rust",
        key: "unexpected_cfgs",
        because: "the workspace declares `cfg(kani)` because kani runs against its crates; this \
                  package has no harness, so declaring that name here would be a second home \
                  for a fact that is not true of it",
    },
];

pub(super) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let workspace = manifest(root, ROOT_MANIFEST)?;
    let desktop = manifest(root, DESKTOP_MANIFEST)?;
    let mut violations = Vec::new();
    metadata(&workspace, &desktop, &mut violations);
    lints(&workspace, &desktop, &mut violations);
    dependencies(&workspace, &desktop, &mut violations);
    quoted(root, &mut violations)?;
    Ok(violations)
}

/// The package metadata, which every member inherits and this package
/// restates.
fn metadata(workspace: &toml::Value, desktop: &toml::Value, out: &mut Vec<Violation>) {
    let inherited = workspace.get("workspace").and_then(|it| it.get("package"));
    let restated = desktop.get("package");
    for key in SHARED_METADATA {
        let expected = inherited.and_then(|table| table.get(key));
        let found = restated.and_then(|table| table.get(key));
        if expected == found {
            continue;
        }
        out.push(diverged(
            format!("{DESKTOP_MANIFEST} [package] {key}"),
            "the out-of-tree package states the metadata every member inherits from \
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
        let copy = desktop.get("lints").and_then(|it| it.get(table));
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
    let location = format!("{DESKTOP_MANIFEST} [lints.{table}] {key}");
    match (recorded, wall == copy) {
        // A decided difference that has become no difference is struck,
        // the way a register entry is struck when its file comes back
        // under budget.
        (Some(row), true) => out.push(diverged(
            format!("xtask/src/guard/wall.rs RECORDED {table}.{key}"),
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
                 and its reason in `RECORDED` (xtask/src/guard/wall.rs) — a difference nobody \
                 wrote down is a wall that fell over quietly"
            ),
        )),
        (Some(_), false) | (None, true) => {}
    }
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

/// The two facts the out-of-tree package quotes from inside the wall:
/// the protocol revision, and the spelling of every error code.
fn quoted(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let decided = revision(root, PROTOCOL_HOME)?;
    let copied = revision(root, PROTOCOL_COPY)?;
    if decided != copied {
        out.push(diverged(
            PROTOCOL_COPY.to_owned(),
            "both ends of this protocol name one revision, so the two constants hold one value",
            format!("`{copied}` against `{decided}` in {PROTOCOL_HOME}"),
            format!(
                "set `PROTOCOL_VERSION` in {PROTOCOL_COPY} to `{decided}`; a server and a \
                 client that disagree here do not finish a handshake"
            ),
        ));
    }
    let defined = codes(root, CODE_HOME)?;
    for minted in codes(root, CODE_QUOTE)?
        .into_iter()
        .filter(|code| !defined.contains(code))
    {
        out.push(diverged(
            CODE_QUOTE.to_owned(),
            "the out-of-tree package quotes the city's error codes and mints none of its own \
             (desktop-SPEC.md section 8.5, first pair)",
            format!("`{minted}` is spelled here and defined nowhere in {CODE_HOME}"),
            format!(
                "add the code to `kernel::AxCode` first, where every consumer of it can read \
                 it, then quote that spelling in {CODE_QUOTE}"
            ),
        ));
    }
    Ok(())
}

/// The protocol revision one file states.
fn revision(root: &Path, rel: &str) -> Result<String, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    text.lines()
        .find_map(|line| {
            let after = line.split_once("PROTOCOL_VERSION: &str =")?;
            quotes(after.1).next()
        })
        .ok_or_else(|| XtaskError::Doc {
            file: rel.to_owned(),
            msg: "this file no longer states `PROTOCOL_VERSION: &str = \"…\"`, which is the \
                  shape both ends of the protocol are compared in"
                .to_owned(),
        })
}

/// Every `E_` code one file spells.
fn codes(root: &Path, rel: &str) -> Result<std::collections::BTreeSet<String>, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    Ok(text
        .lines()
        .flat_map(quotes)
        .filter(|found| found.starts_with("E_"))
        .collect())
}

/// Every double-quoted string on one line.
fn quotes(line: &str) -> impl Iterator<Item = String> + '_ {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(std::borrow::ToOwned::to_owned)
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
