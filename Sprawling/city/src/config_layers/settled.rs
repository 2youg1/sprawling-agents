// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError, Effort, SecondThreshold};

use super::ConfigLayer;
use super::ladder::{Ladder, Layer};

pub fn settled_effort(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(Effort, Layer)>, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .tagged(ConfigLayer::effort)
        .resolve()
        .copied())
}

pub fn settled_second(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(SecondThreshold, Layer)>, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .tagged(ConfigLayer::second_threshold)
        .resolve()
        .copied())
}
