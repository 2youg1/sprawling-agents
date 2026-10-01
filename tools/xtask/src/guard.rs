// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Guard gate: whether a rule this repository states is still the rule
//! it enforces, judged on the tree and never on history.
//!
//! One member, the desktop server's FFI seam, carries a lint table of
//! its own that copies the workspace's in every line but one. A copy
//! drifts without any commit touching both sides, so no rule about
//! commits can see it; `wall` compares the two, key by key, and holds
//! every other member to inheriting the workspace's, on every run.
//!
//! A `Verdict:` trailer on a commit that loosens a gate beside the work it
//! judges stays a rule of AGENTS.md, held by review rather than by this
//! gate: reading history made every gate run depend on the range a caller
//! passed, and the two-commit hole it left open was recorded rather than
//! closed.

use std::path::Path;

use crate::report::{Violation, XtaskError};

mod wall;

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    wall::check(root)
}
