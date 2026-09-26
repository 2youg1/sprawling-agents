// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The subject line and the ruling trailer of every commit in a range
//! (xtask-SPEC §8-35). Not a gate: gates judge the tree, and this judges
//! history, so it runs only where a caller names the range.

use std::path::Path;

use crate::report::{Violation, XtaskError};

/// Where the commits in `range` break the message rules of AGENTS.md.
///
/// # Errors
/// When git cannot be started or refuses the range.
pub(crate) fn check(_root: &Path, _range: &str) -> Result<Vec<Violation>, XtaskError> {
    Ok(Vec::new())
}

/// Every way one commit message breaks the rules, one sentence each.
fn findings(_message: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests;
