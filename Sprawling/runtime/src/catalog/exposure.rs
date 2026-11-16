// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::config::{CatalogExposure, Exposure, ExposureFrom};

use crate::mode::in_some_core;
use crate::tools::{ReadTool, ToolTool};

pub const FIXED: [&str; 2] = [ToolTool::NAME, ReadTool::NAME];

#[must_use]
pub fn exposure_of(
    stated: &CatalogExposure,
    name: &str,
    namespace: &str,
) -> (Exposure, ExposureFrom) {
    let default = fallback(name);
    match default.1 {
        ExposureFrom::Fixed => default,
        ExposureFrom::Default | ExposureFrom::City | ExposureFrom::Building => {
            stated.stated(name, namespace).unwrap_or(default)
        }
    }
}

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
