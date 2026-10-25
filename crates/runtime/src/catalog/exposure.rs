// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The exposure one admitted tool takes in a run
//! (`crates/runtime/spec/Catalog.lean` §8-60, D111): the two tools no
//! table moves, what the building's and the city's `[catalog]` tables
//! state, and the default an unstated tool falls back to.
//!
//! The order the two tables are read in is
//! `kernel::config::CatalogExposure::stated`'s; this module adds the two
//! ends of the ladder that only the catalog knows.

use kernel::config::{CatalogExposure, Exposure, ExposureFrom};

use crate::mode::in_some_core;
use crate::tools::{ReadTool, ToolTool};

/// The tools every session carries whatever a table says: `tool` is the
/// only door to the dormant index, and `read` is the only door to a
/// catalog entry or a skill the index names. Turning either off or down
/// would leave the index listing what no call can reach.
pub const FIXED: [&str; 2] = [ToolTool::NAME, ReadTool::NAME];

/// What `name`, filed under `namespace`, is exposed as in a run frozen
/// with `stated`, and which rung decided it.
#[must_use]
pub fn exposure_of(
    stated: &CatalogExposure,
    name: &str,
    namespace: &str,
) -> (Exposure, ExposureFrom) {
    match fallback(name) {
        fixed @ (_, ExposureFrom::Fixed) => fixed,
        default => stated.stated(name, namespace).unwrap_or(default),
    }
}

/// What `name` is exposed as when no table states it: core for a fixed
/// tool and for a tool in some mode's core (runtime D25), dormant for
/// every other. A page reads it to know what taking a statement out
/// would leave.
#[must_use]
pub fn fallback(name: &str) -> (Exposure, ExposureFrom) {
    if FIXED.contains(&name) {
        (Exposure::Core, ExposureFrom::Fixed)
    } else if in_some_core(name) {
        (Exposure::Core, ExposureFrom::Default)
    } else {
        (Exposure::Dormant, ExposureFrom::Default)
    }
}
