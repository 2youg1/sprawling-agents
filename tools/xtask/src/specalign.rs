// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Gate: kernel enums and the kernel's specification agree variant by
//! variant (C8), and every module's `Spec` anchor resolves. The gate
//! consumes the real enums — `AxCode::ALL` and `EventKind::ALL` — and
//! reads the specification as data, so "the table drifted" and "the enum
//! grew silently" are the same red. Asserts: every AxCode has exactly one
//! arm in the carrier table with its declared carrier; every EventKind
//! exactly one arm in the window table with its window class; every
//! `inductive` in the specification named like a kernel enum holds the
//! variants the kernel compiles; and the seventh module-table column
//! names where the module is specified (`anchors`).
//!
//! The tables and rosters are the Lean shapes `tables` reads out of
//! `crates/kernel/Spec.lean` and its parts (tools/xtask/Spec.lean §8-43);
//! the compiled side is `enums`.

use std::path::Path;

use crate::lean;
use crate::report::{Violation, XtaskError};

mod anchors;
mod enums;
mod tables;

/// The kernel's package directory, whose `Spec.lean` and parts hold the
/// tables this gate reads.
const KERNEL_DIR: &str = "crates/kernel";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let sources = lean::specification(root, KERNEL_DIR)?.ok_or_else(|| XtaskError::Doc {
        file: format!("{KERNEL_DIR}/{}", lean::ENTRY),
        msg: "the kernel has no Lean specification, so there is no table to hold its enums \
              to; the kernel's specification is crates/kernel/Spec.lean and its parts \
              (tools/xtask/Spec.lean section 8-43)"
            .to_owned(),
    })?;
    tables::check(root, &sources, &mut violations)?;
    anchors::check(root, &mut violations)?;
    Ok(violations)
}
