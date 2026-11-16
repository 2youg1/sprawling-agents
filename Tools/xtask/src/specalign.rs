// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::lean;
use crate::report::{Violation, XtaskError};

mod anchors;
mod enums;
mod tables;

const KERNEL_DIR: &str = "Sprawling/kernel";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let sources = lean::specification(root, KERNEL_DIR)?.ok_or_else(|| XtaskError::Doc {
        file: format!("{KERNEL_DIR}/{}", lean::ENTRY),
        msg: "the kernel has no Lean specification, so there is no table to hold its enums \
              to; the kernel's specification is Sprawling/kernel/Spec.lean and its parts \
              (Tools/xtask/Spec.lean section 8-43)"
            .to_owned(),
    })?;
    tables::check(root, &sources, &mut violations)?;
    anchors::check(root, &mut violations)?;
    Ok(violations)
}
