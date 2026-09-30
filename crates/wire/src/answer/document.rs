// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file of the city, as a page reads it.
//!
//! Bounded the way `BuildingDoc` is bounded, and for the same reason:
//! these files are written by agents over months, and an answer that
//! shipped a whole file has nothing to say on the day one of them is a
//! hundred megabytes. A cut is stated, and a file that is not text says
//! so rather than arriving as noise - a ledger segment is text and a
//! content-store blob is not, and both live under the same root.

use kernel::Address;
use serde::{Deserialize, Serialize};

/// One file's head, and what was left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentAnswer {
    pub at: Address,
    /// The file's first bytes as text; empty when `binary`.
    pub text: String,
    /// The whole file's size, whatever `text` carries.
    pub bytes: u64,
    /// Whether `text` stops before the file does.
    pub truncated: bool,
    /// Whether the head held a NUL byte. Such a file is not shown as
    /// text: what a reader would see is not what the file says.
    pub binary: bool,
}
