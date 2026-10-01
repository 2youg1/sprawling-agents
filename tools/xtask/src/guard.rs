// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Guard gate: whether a rule this repository states is still the rule
//! it enforces, judged on the tree and never on history.
//!
//! `desktop/` is a workspace of its own, built outside the root one, and
//! carries a copy of the root workspace's lint table, package metadata
//! and dependency versions. A copy drifts without any commit touching
//! both sides, so no rule about commits can see it; `wall` compares the
//! two, key by key, and holds every package inside the wall to
//! inheriting the copy, on every run.
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
