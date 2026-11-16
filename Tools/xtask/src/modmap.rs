// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::members::{self, Member};
use crate::report::{Violation, XtaskError};
use crate::walk;

mod index;

use index::{check_index_content, is_index_name};

pub(crate) const MAP: &str = "architecture.toml";

const STATUS_PLANNED: &str = "planned";
const STATUSES: [&str; 4] = [STATUS_PLANNED, "building", "built", "frozen"];

const SOURCES: [&str; 2] = ["rs", "zig"];

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

pub(crate) struct Anchor {
    pub(crate) module: String,

    pub(crate) file: String,
    pub(crate) spec: String,
}

fn read_map(root: &Path) -> Result<Map, XtaskError> {
    let text = walk::read_text(&root.join(MAP))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: MAP.to_owned(),
        msg: err.to_string(),
    })
}

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

pub(crate) fn shapes(root: &Path) -> Result<BTreeMap<String, String>, XtaskError> {
    let mut ignored = Vec::new();
    Ok(rows(&read_map(root)?, &judged(root)?, &mut ignored)
        .into_iter()
        .map(|row| (row.path, row.shape))
        .collect())
}

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
        if is_test_module(&rel) && !table.contains_key(rel.as_str()) {
            continue;
        }
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

const GATES: &str = env!("CARGO_PKG_NAME");

fn judged(root: &Path) -> Result<Vec<Member>, XtaskError> {
    Ok(members::members(root)?
        .into_iter()
        .filter(|member| member.package != GATES)
        .collect())
}

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

fn is_source(file: &str) -> bool {
    Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SOURCES.contains(&extension))
}

fn is_test_module(rel: &str) -> bool {
    let Some((_, inside)) = rel.split_once("/src/") else {
        return false;
    };
    let Some(stem) = inside.strip_suffix(".rs") else {
        return false;
    };
    let (dirs, file) = stem.rsplit_once('/').unwrap_or(("", stem));
    file == "tests" || file.ends_with("_tests") || dirs.split('/').any(|dir| dir == "tests")
}

fn is_module(rel: &str) -> bool {
    rel.ends_with(".zig") || rel.contains("/src/")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests;
