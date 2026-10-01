// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The version every one of the workspace's own packages is pinned to
//! (tools/xtask/Spec.lean §8-49).
//!
//! `[workspace.package] version` is the one authority for which release
//! this tree is. A path dependency cannot be published without a
//! `version`, and cargo's dependency tables cannot inherit one, so each
//! of the workspace's own packages carries a pin of its own in
//! `[workspace.dependencies]`: copies, by necessity. **A copy is a
//! second home for a fact**, and nothing else ties these to the
//! version, so this holds every pin to it. A member that names a package
//! by `path` itself would be a pin this table does not see, and is
//! refused for that reason.

use super::{ROOT_MANIFEST, diverged, shown};
use crate::report::Violation;

/// The dependency tables a manifest writes, at its top level and under
/// each `[target.'<cfg>']`.
const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

pub(super) fn pins(workspace: &toml::Value, members: &[(String, toml::Value)]) -> Vec<Violation> {
    let mut out = Vec::new();
    match crate::package::package_field(workspace, "version") {
        Some(version) => pinned(workspace, &format!("={version}"), &mut out),
        None => out.push(diverged(
            format!("{ROOT_MANIFEST} [workspace.package] version"),
            "the workspace states the version its packages are pinned to",
            "absent".to_owned(),
            "state the release this tree is as `version` in [workspace.package]".to_owned(),
        )),
    }
    for (dir, member) in members {
        by_path(dir, member, &mut out);
    }
    out
}

/// Every path dependency of `[workspace.dependencies]`, held to `wanted`.
fn pinned(workspace: &toml::Value, wanted: &str, out: &mut Vec<Violation>) {
    let listed = workspace
        .get("workspace")
        .and_then(|it| it.get("dependencies"))
        .and_then(toml::Value::as_table);
    for (key, dependency) in listed.into_iter().flatten() {
        if dependency.get("path").is_none() {
            continue;
        }
        let written = dependency.get("version");
        if written.and_then(toml::Value::as_str) == Some(wanted) {
            continue;
        }
        out.push(diverged(
            format!("{ROOT_MANIFEST} [workspace.dependencies] {key}"),
            "every package of this workspace is pinned to the workspace's own version",
            format!("{} against `{wanted}`", shown(written)),
            format!(
                "write `version = \"{wanted}\"` in this line: [workspace.package] version is \
                 the one authority, and a path dependency without its pin cannot be published"
            ),
        ));
    }
}

/// Every dependency a member names by `path` rather than through the
/// workspace table, in any of its dependency tables.
fn by_path(dir: &str, member: &toml::Value, out: &mut Vec<Violation>) {
    let mut tables: Vec<(String, &toml::Value)> = TABLES
        .iter()
        .filter_map(|name| member.get(*name).map(|table| (format!("[{name}]"), table)))
        .collect();
    let targets = member.get("target").and_then(toml::Value::as_table);
    for (cfg, target) in targets.into_iter().flatten() {
        tables.extend(TABLES.iter().filter_map(|name| {
            target
                .get(*name)
                .map(|table| (format!("[target.'{cfg}'.{name}]"), table))
        }));
    }
    for (label, table) in tables {
        for (key, dependency) in table.as_table().into_iter().flatten() {
            if dependency.get("path").is_some() {
                out.push(diverged(
                    format!("{dir}/Cargo.toml {label} {key}"),
                    "a package of this workspace is named through [workspace.dependencies] alone",
                    format!(
                        "names a package by its path, {}",
                        shown(dependency.get("path"))
                    ),
                    format!(
                        "write `{key} = {{ workspace = true }}`, with the package's path and \
                         version pin as one line of the root manifest's [workspace.dependencies]"
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests;
