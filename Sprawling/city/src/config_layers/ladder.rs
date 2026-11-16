// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, LayeredValue};

use super::ConfigLayer;
use super::refuse::{READ_A_LAYER, too_near};
use crate::building::Building;

macro_rules! rungs {
    ($($rung:ident),+ $(,)?) => {

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Layer {
            $($rung),+
        }

        impl Layer {

            pub(crate) const ALL: [Layer; [$(Layer::$rung),+].len()] = [$(Layer::$rung),+];
        }
    };
}

rungs!(City, Building, Resident);

impl Layer {

    pub(crate) fn file(self, city_root: &Path, addr: &Address) -> Result<PathBuf, AxError> {
        let layout = CityLayout::new(city_root);
        Ok(match self {

            Layer::City => layout.city_config(),
            Layer::Building => layout.config(Building::of(addr)?.addr()),
            Layer::Resident => layout.config(addr),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ladder {
    rungs: Vec<(Layer, ConfigLayer)>,
}

impl Ladder {

    pub(crate) fn read(city_root: &Path, addr: &Address) -> Result<Ladder, AxError> {
        let mut rungs: Vec<(Layer, ConfigLayer)> = Vec::with_capacity(Layer::ALL.len());
        let mut read: Vec<PathBuf> = Vec::with_capacity(Layer::ALL.len());
        for rung in Layer::ALL {
            let file = rung.file(city_root, addr)?;
            if read.contains(&file) {
                continue;
            }
            rungs.push((rung, stated(&file, rung)?));
            read.push(file);
        }
        Ok(Ladder { rungs })
    }

    pub(crate) fn resolve<T>(&self, stated: impl Fn(&ConfigLayer) -> Option<T>) -> LayeredValue<T> {
        let tagged = self.tagged(stated);
        LayeredValue {
            city: tagged.city.map(|(held, _)| held),
            building: tagged.building.map(|(held, _)| held),
            resident: tagged.resident.map(|(held, _)| held),
        }
    }

    pub(crate) fn tagged<T>(
        &self,
        stated: impl Fn(&ConfigLayer) -> Option<T>,
    ) -> LayeredValue<(T, Layer)> {
        let mut across = LayeredValue::default();
        for (rung, layer) in &self.rungs {
            let held = stated(layer).map(|value| (value, *rung));
            match rung {
                Layer::City => across.city = held,
                Layer::Building => across.building = held,
                Layer::Resident => across.resident = held,
            }
        }
        across
    }
}

pub(crate) fn stated(file: &Path, rung: Layer) -> Result<ConfigLayer, AxError> {
    let stated = match std::fs::read_to_string(file) {

        Ok(text) => ConfigLayer::parse(&text).map_err(|err| in_file(file, &err))?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => ConfigLayer::default(),
        Err(err) => {
            return Err(AxError::failure(
                AxCode::StorageFatal,
                READ_A_LAYER,
                format!("{}: {err}", file.display()),
            )
            .with_recovery("fix the file's permissions; a configuration that exists is read"));
        }
    };
    match stated
        .confined()
        .into_iter()
        .find(|table| !table.reached_from(rung))
    {
        Some(table) => Err(too_near(file, table)),
        None => Ok(stated),
    }
}

fn in_file(file: &Path, refused: &AxError) -> AxError {
    AxError::failure(
        *refused.code(),
        refused.action(),
        format!("{}: {}", file.display(), refused.subject()),
    )
    .with_nearby(refused.nearby().to_vec())
    .with_recovery(refused.recovery())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
