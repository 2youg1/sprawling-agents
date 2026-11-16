// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use super::{diverged, shown};
use crate::report::Violation;

const LEAVES: [&str; 2] = ["Sprawling/desktop/ffi", "Calyx/ffi"];

struct Recorded {
    table: &'static str,
    key: &'static str,
    because: &'static str,
}

const RECORDED: [Recorded; 1] = [Recorded {
    table: "rust",
    key: "unsafe_code",
    because: "each call into a Zig leaf relaxes it at that one statement with a written \
              reason, which `forbid` makes impossible (`Sprawling/desktop/Spec.lean` section 12.14, \
              `Calyx/ffi/Spec.lean` D1)",
}];

pub(super) fn tables(workspace: &toml::Value, members: &[(String, toml::Value)]) -> Vec<Violation> {
    let mut out = Vec::new();
    let wall = workspace.get("workspace").and_then(|it| it.get("lints"));
    for leaf in LEAVES {
        match members.iter().find(|(dir, _)| dir == leaf) {
            Some((_, manifest)) => compared(wall, (leaf, manifest.get("lints")), &mut out),
            None => out.push(diverged(
                format!("Tools/xtask/src/guard/wall.rs LEAVES {leaf}"),
                "a lint table of its own belongs to a workspace member",
                format!("no workspace member lives at {leaf}"),
                format!(
                    "strike {leaf} from `LEAVES`, and its rows in `RECORDED` once no leaf is left: \
                     the exception they grant has nothing left to be granted to"
                ),
            )),
        }
    }
    for (dir, member) in members
        .iter()
        .filter(|(dir, _)| !LEAVES.contains(&dir.as_str()))
    {
        if !inheriting(member.get("lints")) {
            out.push(diverged(
                format!("{dir}/Cargo.toml [lints]"),
                "every member but the leaves inherits the workspace's lint table",
                format!("{} against `workspace = true`", shown(member.get("lints"))),
                format!(
                    "write `[lints]` in {dir}/Cargo.toml as `workspace = true` and nothing else"
                ),
            ));
        }
    }
    out
}

fn compared(
    wall: Option<&toml::Value>,
    (leaf, copy): (&str, Option<&toml::Value>),
    out: &mut Vec<Violation>,
) {
    for table in ["rust", "clippy"] {
        let wall = wall.and_then(|it| it.get(table));
        let copy = copy.and_then(|it| it.get(table));
        let mut named = keys(wall);
        named.extend(keys(copy));
        for key in named {
            judge(
                Compared {
                    leaf,
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

struct Compared<'a> {
    leaf: &'a str,
    table: &'a str,
    key: &'a str,
    wall: Option<&'a toml::Value>,
    copy: Option<&'a toml::Value>,
}

fn judge(compared: Compared<'_>, out: &mut Vec<Violation>) {
    let Compared {
        leaf,
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
            format!("Tools/xtask/src/guard/wall.rs RECORDED {table}.{key}"),
            "a recorded difference between the two tables is struck once the two sides agree",
            format!("`{key}` now reads the same on both sides"),
            format!(
                "delete its row: it was granted because {}, and that reason no longer shows",
                row.because
            ),
        )),
        (None, false) => out.push(diverged(
            format!("{leaf}/Cargo.toml [lints.{table}] {key}"),
            "the leaf's lint table is the workspace's own, key for key",
            format!("{} against the workspace's {}", shown(copy), shown(wall)),
            format!(
                "copy the workspace's line into {leaf}/Cargo.toml, or record the difference \
                 and its reason in `RECORDED` (Tools/xtask/src/guard/wall.rs) — a difference \
                 nobody wrote down is a wall that fell over quietly"
            ),
        )),
        (Some(_), false) | (None, true) => {}
    }
}

fn inheriting(value: Option<&toml::Value>) -> bool {
    value.and_then(toml::Value::as_table).is_some_and(|table| {
        table.len() == 1 && table.get("workspace") == Some(&toml::Value::Boolean(true))
    })
}

fn keys(table: Option<&toml::Value>) -> BTreeSet<String> {
    match table.and_then(toml::Value::as_table) {
        Some(found) => found.keys().cloned().collect(),
        None => BTreeSet::new(),
    }
}

#[cfg(test)]
mod tests;
