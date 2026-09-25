// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a session branched off.

use serde::{Deserialize, Serialize};

use crate::event::{RunId, Seq};

/// Where a session branched off: one run of this city, and the line in
/// it that the branch starts after.
///
/// **The pair has one home, and its older spelling stays where it is.**
/// A `run_forked` record spells this same pair as `{from, at_seq}`,
/// because a ledger already written cannot be re-spelled; a value this
/// build passes around is the type below. A reader holding a `RunForked`
/// builds an `Origin` from it rather than keeping both.
///
/// `at_seq` is the line itself and not the line after it: a branch
/// inherits through that line, and the safe point the rebuild settles on
/// is that value or an earlier one (`runtime::fork::inherited`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Origin {
    /// The run the branch came from.
    pub run: RunId,
    /// The last line of it the branch inherits.
    pub at_seq: Seq,
}
