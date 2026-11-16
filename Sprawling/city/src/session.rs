// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError};

use crate::config_layers;
use crate::spine_files;

pub fn clear_session(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    forget_shape(city_root, addr)?;
    spine_files::clear_handoff(city_root, addr)
}

pub fn forget_shape(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    config_layers::forget_session(city_root, addr)?;
    config_layers::forget_catalog_session(city_root, addr)
}

#[cfg(test)]
mod tests;
