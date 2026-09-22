// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Module-map gate (redline C4). `architecture.toml` is a closed list:
//! every `.rs` under `crates/*/src` is either a registered module, a
//! `lib.rs`, or a pure index file. Status coherence is checked both ways —
//! a file that exists while its entry still says planned means the builder
//! skipped the "flip the status" leg of the completion evidence.
//!
//! **The map is data, and used to be prose.** It was a seven-column table
//! inside `ARCHITECTURE.md`, which cost that document seven hundred lines
//! nobody reads through and gave every entry a *position* that a person
//! maintained: each heading stated how many rows sat under it, and that
//! count could go stale while the rows were right. A structured file has
//! no positions to keep, and a reader can ask it for one module instead of
//! scanning for one.
//!
//! **`lib.rs` and pure index files are exempt, and that exemption is
//! checked** rather than trusted: an index file may hold declarations and
//! nothing else, so exempting it costs no coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The module map. This gate is its only reader, so a field change breaks
/// one place (xtask-SPEC.md section 8-10); `specalign` names the file when
/// reporting an anchor it could not resolve.
pub(crate) const MAP: &str = "architecture.toml";
/// The status column, in the language the table is written in. It moved
/// from Chinese to English when the module table became part of what the
/// repository publishes: a reader of `ARCHITECTURE.md` should not have to
/// read a second language to find out whether a module exists.
const STATUS_PLANNED: &str = "planned";
const STATUSES: [&str; 4] = [STATUS_PLANNED, "building", "built", "frozen"];

/// Line prefixes allowed in `lib.rs` and pure index files: single-line
/// declarations only, which is what makes exempting them free.
const INDEX_PREFIXES: [&str; 8] = [
    "//",
    "#![",
    "#[",
    "pub mod ",
    "mod ",
    "pub use ",
    "pub(crate) use ",
    "use ",
];

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
/// the code says". This gate stays the map's only reader (xtask-SPEC.md
/// section 8-10), so a field change breaks one place. The module names
/// itself rather than carrying a line number: an entry is found by name in
/// a structured file, and a name does not move when the file is reordered.
pub(crate) struct Anchor {
    pub(crate) module: String,
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
    Ok(rows(&read_map(root)?, &mut ignored)
        .into_iter()
        .map(|row| Anchor {
            module: row.module,
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
    Ok(rows(&read_map(root)?, &mut ignored)
        .into_iter()
        .map(|row| (row.path, row.shape))
        .collect())
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let rows = rows(&read_map(root)?, &mut violations);

    let table: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.path.as_str(), row)).collect();
    // Directories that hold registered modules; `<dir>.rs` is then an index file.
    let index_dirs: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| row.path.rsplit_once('/').map(|(dir, _)| dir.to_owned()))
        .collect();

    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    for file in walk::files_with_ext(&root.join("crates"), &["rs"])? {
        let rel = walk::rel(root, &file);
        if !rel.contains("/src/") {
            continue; // build.rs and friends are not modules
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

/// The entries this gate judges: a `::` in the name, a `crates/**.rs`
/// file, a known status, an `owns` that says something, and one entry per
/// file.
///
/// An entry whose file sits outside `crates/` is carried by the map for a
/// reader and skipped here, because the walk below only reaches `crates/`
/// \u2014 `desktop` is built out of tree and its files are not this gate's to
/// judge.
fn rows(map: &Map, violations: &mut Vec<Violation>) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in &map.module {
        let (name, file) = (entry.name.as_str(), entry.file.as_str());
        if !name.contains("::") || !file.starts_with("crates/") || !file.ends_with(".rs") {
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

fn is_index_name(rel: &str, index_dirs: &BTreeSet<String>) -> bool {
    if rel.ends_with("/lib.rs") {
        return true;
    }
    rel.strip_suffix(".rs")
        .is_some_and(|stem| index_dirs.contains(stem))
}

fn check_index_content(
    root: &Path,
    rel: &str,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let text = walk::read_text(&root.join(rel))?;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let allowed =
            line.is_empty() || INDEX_PREFIXES.iter().any(|prefix| line.starts_with(prefix));
        if !allowed {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{rel}:{}", index.saturating_add(1)),
                rule: "index files hold declarations only — comments, attributes, \
                       mod, use"
                    .to_owned(),
                violation: format!("logic line in an index file: {line:?}"),
                alternative: "move the logic into a registered module".to_owned(),
            });
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests;
