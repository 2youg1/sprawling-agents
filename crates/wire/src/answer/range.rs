// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One window of a stored document version (wire-SPEC.md 8-70).
//!
//! Read by the version rather than by the path, so a page three screens
//! into a file a resident is rewriting goes on reading the version it
//! opened. The window is the one the `documents` crate cut: `window.span`
//! says where it really starts and ends, which is where the next request
//! starts from.

use documents::Window;
use kernel::B3Hash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RangeAnswer {
    pub version: B3Hash,
    pub window: Window,
}
