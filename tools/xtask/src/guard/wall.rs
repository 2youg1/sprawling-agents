// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The workspace's lint wall, and the one table that stands beside it.
//!
//! Every member inherits `[workspace.lints]` by writing `[lints]
//! workspace = true`, except the desktop server's FFI seam,
//! `crates/desktop/ffi`. Each call into its Zig leaf relaxes
//! `unsafe_code` at that one statement, which `forbid` makes impossible,
//! so it writes a table of its own: the workspace's, with `unsafe_code`
//! at `deny` (`crates/desktop/Spec.lean` D14, xtask-SPEC.md section 8-46).
//! Every other line of that table is a **copy**, and a copy is a second
//! home for a fact.
//!
//! **This is the shape no rule about commits can see.** One side is
//! edited, the other is not, and nothing red follows. So the copy is
//! compared here, key by key, and a difference is a refusal unless it is
//! one this module records with its reason. A member that writes a table
//! of its own stands outside the comparison, so every other member is
//! held to inheriting.
//!
//! **A recorded difference cleans itself.** A row whose two sides have
//! become equal is struck, exactly as `length` strikes a register entry
//! for a file that came back under budget, and a leaf that is no longer
//! a member strikes its row too: an exception nobody needs is an
//! exception nobody decided to keep granting.

use std::collections::BTreeSet;

use super::{diverged, shown};
use crate::report::Violation;

/// The one member whose lint table is its own.
const LEAF: &str = "crates/desktop/ffi";

/// One key the leaf's table is allowed to disagree with the workspace
/// about, and why.
///
/// The reason is printed when the disagreement disappears, so whoever
/// strikes the row reads what it was for.
struct Recorded {
    table: &'static str,
    key: &'static str,
    because: &'static str,
}

/// Every difference between the two tables that somebody decided.
///
/// Nothing else may differ. Adding a row here is a re-pricing of the
/// rule and lands in `tools/xtask/`, in a commit of its own.
const RECORDED: [Recorded; 1] = [Recorded {
    table: "rust",
    key: "unsafe_code",
    because: "each call into the Zig leaf that carries the Win32 calls relaxes it at that one \
              statement with a written reason, which `forbid` makes impossible (`crates/desktop/Spec.lean` \
              section 12.14)",
}];

/// Every member's lint table, judged against the workspace's: the leaf
/// key by key, every other member for inheriting it.
pub(super) fn tables(workspace: &toml::Value, members: &[(String, toml::Value)]) -> Vec<Violation> {
    let mut out = Vec::new();
    let wall = workspace.get("workspace").and_then(|it| it.get("lints"));
    match members.iter().find(|(dir, _)| dir == LEAF) {
        Some((_, leaf)) => compared(wall, leaf.get("lints"), &mut out),
        None => out.push(diverged(
            format!("tools/xtask/src/guard/wall.rs LEAF {LEAF}"),
            "the one lint table of its own belongs to a workspace member",
            format!("no workspace member lives at {LEAF}"),
            "strike `LEAF` and its rows in `RECORDED`: the exception they grant has nothing left \
             to be granted to"
                .to_owned(),
        )),
    }
    for (dir, member) in members.iter().filter(|(dir, _)| dir != LEAF) {
        if !inheriting(member.get("lints")) {
            out.push(diverged(
                format!("{dir}/Cargo.toml [lints]"),
                "every member but the leaf inherits the workspace's lint table",
                format!("{} against `workspace = true`", shown(member.get("lints"))),
                format!(
                    "write `[lints]` in {dir}/Cargo.toml as `workspace = true` and nothing else"
                ),
            ));
        }
    }
    out
}

/// The two tables, key by key, in both directions.
fn compared(wall: Option<&toml::Value>, copy: Option<&toml::Value>, out: &mut Vec<Violation>) {
    for table in ["rust", "clippy"] {
        let wall = wall.and_then(|it| it.get(table));
        let copy = copy.and_then(|it| it.get(table));
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

/// One lint key as the two tables spell it.
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
    match (recorded, wall == copy) {
        // A decided difference that has become no difference is struck,
        // the way a register entry is struck when its file comes back
        // under budget.
        (Some(row), true) => out.push(diverged(
            format!("tools/xtask/src/guard/wall.rs RECORDED {table}.{key}"),
            "a recorded difference between the two tables is struck once the two sides agree",
            format!("`{key}` now reads the same on both sides"),
            format!(
                "delete its row: it was granted because {}, and that reason no longer shows",
                row.because
            ),
        )),
        (None, false) => out.push(diverged(
            format!("{LEAF}/Cargo.toml [lints.{table}] {key}"),
            "the leaf's lint table is the workspace's own, key for key",
            format!("{} against the workspace's {}", shown(copy), shown(wall)),
            format!(
                "copy the workspace's line into {LEAF}/Cargo.toml, or record the difference \
                 and its reason in `RECORDED` (tools/xtask/src/guard/wall.rs) — a difference \
                 nobody wrote down is a wall that fell over quietly"
            ),
        )),
        (Some(_), false) | (None, true) => {}
    }
}

/// Whether a value is a table holding `workspace = true` and nothing
/// else.
fn inheriting(value: Option<&toml::Value>) -> bool {
    value.and_then(toml::Value::as_table).is_some_and(|table| {
        table.len() == 1 && table.get("workspace") == Some(&toml::Value::Boolean(true))
    })
}

/// The keys of one table. A table that is absent has no keys, which is
/// the reading that lets a key present on one side only be compared
/// against nothing on the other.
fn keys(table: Option<&toml::Value>) -> BTreeSet<String> {
    match table.and_then(toml::Value::as_table) {
        Some(found) => found.keys().cloned().collect(),
        None => BTreeSet::new(),
    }
}

#[cfg(test)]
mod tests;
