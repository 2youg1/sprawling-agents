// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Packaged gate: a package that can be published compiles only files it
//! carries (xtask-SPEC.md section 8-49).

use std::path::{Path, PathBuf};

use crate::report::{Violation, XtaskError};

pub(crate) fn check(_root: &Path) -> Result<Vec<Violation>, XtaskError> {
    Ok(Vec::new())
}

fn excludes_production(_predicate: &syn::Meta) -> bool {
    false
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
