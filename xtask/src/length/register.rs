// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `xtask/budgets.toml` states about length, read once and handed
//! to the gate as values.
//!
//! The three limits and the two registers of exceptions all arrive from
//! one file, and every message the gate prints names the row it came
//! from. Keeping the row names and the reader together means a renamed
//! row is renamed in one place, and the gate above reads as judgement
//! rather than as table lookup.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::XtaskError;

/// The register file every limit below is read from, named in full
/// because each violation tells the reader which file to edit.
const REGISTER: &str = "xtask/budgets.toml";

/// The register row that states how long a production function may be.
pub(super) const ROW: &str = "function_length";

/// The register row that states how long a file may be.
pub(super) const FILE_ROW: &str = "file_length";

/// The sub-table naming what was already over the line when it was
/// drawn: for files, the length each had that day; for signatures, the
/// names that were already too wide.
pub(super) const PREDATING: &str = "predating";

/// The register row that states how many parameters a function may take.
pub(super) const ARG_ROW: &str = "argument_count";

/// The number one row states, whether it is stated in lines or in
/// parameters.
pub(super) fn limit(root: &Path, row: &str) -> Result<usize, XtaskError> {
    let register = crate::budget::register(root)?;
    let stated = register
        .get(row)
        .and_then(|found| {
            found
                .get("budget_lines")
                .or_else(|| found.get("budget_arguments"))
        })
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("{row} states no budget_lines"),
        })?;
    usize::try_from(stated).map_err(|_| XtaskError::Doc {
        file: REGISTER.to_owned(),
        msg: format!("{row}.budget_lines is not a line count: {stated}"),
    })
}

/// The files that were already over the line when it was drawn, each
/// pinned at the length it had that day.
pub(super) fn predating(root: &Path) -> Result<BTreeMap<String, usize>, XtaskError> {
    let register = crate::budget::register(root)?;
    let Some(table) = register
        .get(FILE_ROW)
        .and_then(|row| row.get(PREDATING))
        .and_then(toml::Value::as_table)
    else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for (rel, value) in table {
        let stated = value.as_integer().ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("{FILE_ROW}.{PREDATING}.{rel} is not a line count"),
        })?;
        let pinned = usize::try_from(stated).map_err(|_| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("{FILE_ROW}.{PREDATING}.{rel} is not a line count: {stated}"),
        })?;
        out.insert(rel.clone(), pinned);
    }
    Ok(out)
}

/// The functions whose parameter lists predate the rule.
pub(super) fn excused(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let register = crate::budget::register(root)?;
    let Some(listed) = register
        .get(ARG_ROW)
        .and_then(|row| row.get(PREDATING))
        .and_then(|table| table.get("names"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(BTreeSet::new());
    };
    let mut out = BTreeSet::new();
    for value in listed {
        let named = value.as_str().ok_or_else(|| XtaskError::Doc {
            file: REGISTER.to_owned(),
            msg: format!("{ARG_ROW}.{PREDATING} holds something that is not a name"),
        })?;
        out.insert(named.to_owned());
    }
    Ok(out)
}

/// How a function is named in the register: the file it lives in and its
/// own name. Two functions in one file cannot share a name, and a name
/// alone would excuse every `new` in the workspace at once.
pub(super) fn key(rel: &str, name: &str) -> String {
    format!("{rel}::{name}")
}
