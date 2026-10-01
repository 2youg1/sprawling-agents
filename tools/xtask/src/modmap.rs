// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Module-map gate (redline C4). `architecture.toml` is a closed list:
//! every `.rs` under the `src/` of every package but the gates' own is
//! either a registered module, a `lib.rs`, or a pure index file, and
//! every `.zig` anywhere in such a package is a registered module. Status coherence
//! is checked both ways — a file that exists while its entry still says
//! planned means the builder skipped the "flip the status" leg of the
//! completion evidence.
//!
//! **The map is data, not prose.** A table inside `ARCHITECTURE.md`
//! would give every entry a *position* a person maintains, and a count
//! under each heading that can go stale while the rows are right. A
//! structured file has no positions to keep, and a reader can ask it for
//! one module instead of scanning for one.
//!
//! **`lib.rs` and pure index files are exempt, and that exemption is
//! checked** rather than trusted: an index file may hold declarations and
//! nothing else, so exempting it costs no coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::members::{self, Member};
use crate::report::{Violation, XtaskError};
use crate::walk;

mod index;

use index::{check_index_content, is_index_name};

/// The module map. This gate is its only reader, so a field change breaks
/// one place (tools/xtask/Spec.lean §8-10); `specalign` names the file when
/// reporting an anchor it could not resolve.
pub(crate) const MAP: &str = "architecture.toml";
/// The status column, in the language the table is written in. It moved
/// from Chinese to English when the module table became part of what the
/// repository publishes: a reader of `ARCHITECTURE.md` should not have to
/// read a second language to find out whether a module exists.
const STATUS_PLANNED: &str = "planned";
const STATUSES: [&str; 4] = [STATUS_PLANNED, "building", "built", "frozen"];

/// The languages whose files are modules, by extension. A Zig leaf is
/// admitted only when this gate reads its files (ARCHITECTURE.md
/// section 2, condition 5).
const SOURCES: [&str; 2] = ["rs", "zig"];

/// One module's entry, exactly as the map spells it.
///
/// `owns` is read only to refuse an empty one: an entry that does not say
/// what its module is for has stopped being worth its line. `since` is
/// carried by the file for a person and by no field here, because no gate
/// has a use for the stage a module arrived in.
#[derive(Deserialize)]
struct Entry {
    name: String,
    file: String,
    owns: String,
    shape: String,
    status: String,
    spec: String,
}

#[derive(Deserialize)]
struct Map {
    module: Vec<Entry>,
    /// What each family of modules is for, keyed by the prefix its module
    /// names carry; read by [`duties`] alone.
    #[serde(default)]
    family: BTreeMap<String, Family>,
}

#[derive(Deserialize)]
struct Family {
    duty: String,
}

struct Row {
    module: String,
    path: String,
    shape: String,
    status: String,
    spec: String,
}

/// One module's claim about where its interface is specified.
///
/// Handed to `specalign`, which is the gate that owns "a SPEC says what
/// the code says". This gate stays the map's only reader (tools/xtask/Spec.lean
/// §8-10), so a field change breaks one place. The module names
/// itself rather than carrying a line number: an entry is found by name in
/// a structured file, and a name does not move when the file is reordered.
pub(crate) struct Anchor {
    pub(crate) module: String,
    /// The row's file, which says which package the row belongs to.
    pub(crate) file: String,
    pub(crate) spec: String,
}

/// The map, parsed.
///
/// # Errors
/// When the file is missing or will not parse: a gate whose authority has
/// moved says so rather than reporting every file in the tree as
/// unregistered.
fn read_map(root: &Path) -> Result<Map, XtaskError> {
    let text = walk::read_text(&root.join(MAP))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: MAP.to_owned(),
        msg: err.to_string(),
    })
}

/// Every registered module's `spec`, in the order the map states them.
pub(crate) fn anchors(root: &Path) -> Result<Vec<Anchor>, XtaskError> {
    let mut ignored = Vec::new();
    Ok(rows(&read_map(root)?, &judged(root)?, &mut ignored)
        .into_iter()
        .map(|row| Anchor {
            module: row.module,
            file: row.path,
            spec: row.spec,
        })
        .collect())
}

/// The shape each registered module states, by file path.
///
/// Read by `length`, which does not measure a module whose shape is
/// `data`. It comes from this parser rather than a second one: the
/// module table has one reader, so a change to its columns breaks one
/// place.
pub(crate) fn shapes(root: &Path) -> Result<BTreeMap<String, String>, XtaskError> {
    let mut ignored = Vec::new();
    Ok(rows(&read_map(root)?, &judged(root)?, &mut ignored)
        .into_iter()
        .map(|row| (row.path, row.shape))
        .collect())
}

/// What each of `packages` owns, in their order: the `duty` of the family
/// its registered modules are named for (xtask D5).
///
/// # Errors
/// When the map does not parse, and when a package has no registered
/// module, has modules named for two families, or is named for a family
/// the map states no duty for: a table that quietly left the cell empty
/// would read as a crate that owns nothing.
pub(crate) fn duties(root: &Path, packages: &[Member]) -> Result<Vec<String>, XtaskError> {
    let map = read_map(root)?;
    packages
        .iter()
        .map(|member| {
            let family = family_of(&map, member)?;
            map.family
                .get(family)
                .map(|found| found.duty.clone())
                .ok_or_else(|| XtaskError::Doc {
                    file: MAP.to_owned(),
                    msg: format!(
                        "no `[family.{family}]` duty, so nothing says what {} owns; add the \
                         family with one clause of duty",
                        member.package
                    ),
                })
        })
        .collect()
}

/// The one prefix, before the first `::`, that every registered module of
/// `member` carries.
fn family_of<'map>(map: &'map Map, member: &Member) -> Result<&'map str, XtaskError> {
    let prefixes: Vec<&str> = map
        .module
        .iter()
        .filter(|entry| member.holds(&entry.file))
        .filter_map(|entry| entry.name.split_once("::").map(|(prefix, _)| prefix))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    match *prefixes.as_slice() {
        [family] => Ok(family),
        [] => Err(XtaskError::Doc {
            file: MAP.to_owned(),
            msg: format!(
                "no module is registered under {}, so no family says what {} owns; register \
                 its modules first",
                member.dir, member.package
            ),
        }),
        [_, _, ..] => Err(XtaskError::Doc {
            file: MAP.to_owned(),
            msg: format!(
                "the modules under {} are named for {} families, so which one {} belongs to is \
                 not stated; name them for one family",
                member.dir,
                prefixes.join(", "),
                member.package
            ),
        }),
    }
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let scope = judged(root)?;
    let rows = rows(&read_map(root)?, &scope, &mut violations);

    let table: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.path.as_str(), row)).collect();
    // Directories that hold registered modules; `<dir>.rs` is then an index file.
    let index_dirs: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| row.path.rsplit_once('/').map(|(dir, _)| dir.to_owned()))
        .collect();

    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    let files = walk::files_under(
        root,
        scope.iter().map(|member| member.dir.as_str()),
        &SOURCES,
    )?;
    for file in files {
        let rel = walk::rel(root, &file);
        if !is_module(&rel) {
            continue;
        }
        on_disk.insert(rel.clone());
        if let Some(row) = table.get(rel.as_str()) {
            if row.status == STATUS_PLANNED {
                violations.push(Violation {
                    gate: "modmap",
                    location: rel,
                    rule: format!(
                        "completion evidence includes flipping the module status ({MAP})"
                    ),
                    violation: format!(
                        "file exists but the entry for {} still says {STATUS_PLANNED}",
                        row.module
                    ),
                    alternative: "flip the status in the same change-set, or delete the file"
                        .to_owned(),
                });
            }
        } else if is_index_name(&rel, &index_dirs) {
            check_index_content(root, &rel, &mut violations)?;
        } else {
            violations.push(Violation {
                gate: "modmap",
                location: rel,
                rule: format!("the module map is a closed list ({MAP}, C4)"),
                violation: "file is not registered in the module map".to_owned(),
                alternative: format!(
                    "add an entry (name/file/owns/shape/since/status/spec) to {MAP} \
                     first, or delete the file"
                ),
            });
        }
    }

    for row in &rows {
        if row.status != STATUS_PLANNED && !on_disk.contains(&row.path) {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{MAP}: {}", row.module),
                rule: "a non-planned status claims the file exists".to_owned(),
                violation: format!("{} is marked {} but missing on disk", row.path, row.status),
                alternative: format!(
                    "create {} or set the entry back to {STATUS_PLANNED}",
                    row.path
                ),
            });
        }
    }
    Ok(violations)
}

/// The package these gates are compiled into. Its modules are described
/// by tools/xtask/Spec.lean §7 rather than by the map, and it is the one
/// package the map does not cover (section 12-9).
const GATES: &str = env!("CARGO_PKG_NAME");

/// The packages whose files the map registers: every package the
/// members reader lists, tools included,
/// except the gates' own. A new tool is covered from its first file, so
/// forgetting to register one is a red rather than a package quietly
/// left unjudged.
fn judged(root: &Path) -> Result<Vec<Member>, XtaskError> {
    Ok(members::members(root)?
        .into_iter()
        .filter(|member| member.package != GATES)
        .collect())
}

/// The entries this gate judges: a `::` in the name, a `.rs` file inside
/// one of `scope`'s packages, a known status, an `owns` that says
/// something, and one entry per file.
///
/// An entry whose file sits in no package of `scope` is skipped here,
/// because the walk only reaches the same packages.
fn rows(map: &Map, scope: &[Member], violations: &mut Vec<Violation>) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in &map.module {
        let (name, file) = (entry.name.as_str(), entry.file.as_str());
        let judged = scope.iter().any(|member| member.holds(file));
        if !name.contains("::") || !judged || !is_source(file) {
            continue;
        }
        if !STATUSES.contains(&entry.status.as_str()) {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{MAP}: {name}"),
                rule: "status is one of planned|building|built|frozen".to_owned(),
                violation: format!("entry for {file} has status {:?}", entry.status),
                alternative: "use a value from the status enum".to_owned(),
            });
            continue;
        }
        if entry.owns.trim().is_empty() {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{MAP}: {name}"),
                rule: "an entry says what its module owns".to_owned(),
                violation: format!("{name} has an empty `owns`"),
                alternative: "state the duty in one clause, or delete the entry".to_owned(),
            });
            continue;
        }
        if let Some(first) = seen.get(file) {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{MAP}: {name}"),
                rule: "one module, one entry".to_owned(),
                violation: format!("{file} is already registered as {first}"),
                alternative: "merge the duplicate entries".to_owned(),
            });
            continue;
        }
        seen.insert(file, name);
        rows.push(Row {
            module: entry.name.clone(),
            path: entry.file.clone(),
            shape: entry.shape.clone(),
            status: entry.status.clone(),
            spec: entry.spec.clone(),
        });
    }
    rows
}

/// Whether a file is written in one of the languages this map registers.
fn is_source(file: &str) -> bool {
    Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SOURCES.contains(&extension))
}

/// Whether a source file found in a package is a module. A Zig file is,
/// wherever it sits, because a Zig leaf keeps its sources beside its
/// manifest; a Rust file is when it is under `src/`, because `build.rs`
/// and its friends are not modules.
fn is_module(rel: &str) -> bool {
    rel.ends_with(".zig") || rel.contains("/src/")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests;
