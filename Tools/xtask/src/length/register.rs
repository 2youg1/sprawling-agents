// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use crate::budget::REGISTER;
use crate::report::XtaskError;

pub(super) const ROW: &str = "function_length";

pub(super) const PREDATING: &str = "predating";

pub(super) const ARG_ROW: &str = "argument_count";

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

pub(super) fn key(rel: &str, name: &str) -> String {
    format!("{rel}::{name}")
}
