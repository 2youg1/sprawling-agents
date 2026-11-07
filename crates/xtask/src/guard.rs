// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Guard gate: whether a rule this repository states is still the rule
//! it enforces, judged on the tree and never on history.
//!
//! Two copies in the manifests have no rule about commits that could
//! see them drift. One member, the desktop server's FFI seam, carries a
//! lint table of its own that copies the workspace's in every line but
//! one; `wall` compares the two, key by key, and holds every other member
//! to inheriting the workspace's. And every one of the workspace's own
//! packages is pinned in the root manifest to the workspace's version,
//! because a path dependency without one cannot be published; `version`
//! holds each pin to that version and every member to taking the
//! package through the root manifest.
//!
//! A `Verdict:` trailer on a commit that loosens a gate beside the work it
//! judges stays a rule of AGENTS.md, held by review rather than by this
//! gate: reading history made every gate run depend on the range a caller
//! passed, and the two-commit hole it left open was recorded rather than
//! closed.

use std::path::Path;

use crate::members;
use crate::report::{Violation, XtaskError};

mod version;
mod wall;

const ROOT_MANIFEST: &str = "Cargo.toml";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let workspace = manifest(root, ROOT_MANIFEST)?;
    let manifests = members::members(root)?
        .into_iter()
        .map(|member| {
            manifest(root, &format!("{}/Cargo.toml", member.dir)).map(|read| (member.dir, read))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut found = wall::tables(&workspace, &manifests);
    found.extend(version::pins(&workspace, &manifests));
    Ok(found)
}

/// One manifest, read as a value.
fn manifest(root: &Path, rel: &str) -> Result<toml::Value, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    toml::from_str(&text).map_err(|err| XtaskError::Doc {
        file: rel.to_owned(),
        msg: format!("this manifest does not parse as TOML: {err}"),
    })
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
