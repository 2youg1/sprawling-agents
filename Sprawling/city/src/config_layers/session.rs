// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError, B3Hash, Effort, FrozenGeneration};

use super::write::{Change, change};
use super::{ConfigLayer, Layer, path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionShape {
    pub model: String,
    pub endpoint: Option<String>,
    pub effort: Option<Effort>,
    pub generation: Option<FrozenGeneration>,
}

impl SessionShape {
    pub fn new(
        model: impl Into<String>,
        effort: Option<Effort>,
        generation: Option<FrozenGeneration>,
    ) -> SessionShape {
        SessionShape {
            model: model.into(),
            endpoint: None,
            effort,
            generation,
        }
    }
}

pub fn own_layer(city_root: &Path, addr: &Address) -> Result<ConfigLayer, AxError> {
    let rung = if crate::building::Building::of(addr)?.addr() == addr {
        Layer::Building
    } else {
        Layer::Resident
    };
    super::ladder::stated(&path(city_root, addr, Layer::Resident)?, rung)
}

pub fn write_session(
    city_root: &Path,
    addr: &Address,
    shape: &SessionShape,
) -> Result<(), AxError> {
    change(
        city_root,
        addr,
        Layer::Resident,
        Change::Session {
            model: &shape.model,
            endpoint: shape.endpoint.as_deref(),
            effort: shape.effort,
            generation: shape.generation.as_ref(),
        },
    )
}

pub fn freeze_naming(city_root: &Path, addr: &Address, version: B3Hash) -> Result<(), AxError> {
    change(city_root, addr, Layer::Resident, Change::Naming(version))
}

pub(crate) fn forget(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    change(city_root, addr, Layer::Resident, Change::Forget)
}

#[cfg(test)]
mod tests;
