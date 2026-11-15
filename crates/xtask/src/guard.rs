// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `wall` compares manifest lint tables with the workspace; `version`
//! checks workspace package pins. Gate changes fix code, narrow a precise
//! scope or delete a rule, never add an exception. No history is read.

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

/// A lint value or `absent`, rendered for a diagnostic.
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
