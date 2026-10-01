// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One window of a stored Markdown version, laid out as blocks
//! (wire-SPEC.md 8-74).
//!
//! Read by the version, for the reason a range is: a page previewing a
//! file a resident is rewriting goes on reading the version it opened.
//! The blocks are the `documents` crate's grammar, carried as it spells
//! them (`crates/documents/Spec.lean` D1, D21); `Laid.span` says which
//! stretch was read, which is where the next request starts from.

use documents::Preview;
use kernel::B3Hash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PreviewAnswer {
    pub version: B3Hash,
    pub preview: Preview,
}
