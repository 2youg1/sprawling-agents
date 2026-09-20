// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ordered stack of configuration layers a run is governed by, as
//! one value (city-SPEC.md section 8-4).
//!
//! A rung is a scope that may state a value, and the ladder is every
//! rung that exists for one address, read once, in order from the
//! farthest to the nearest. Before this module each concern was
//! resolved by naming the three scopes again — one read per scope, one
//! field per scope, three times over — so adding a rung meant editing
//! every concern, and a rung added to one concern and forgotten in the
//! next would have been a silent difference in what a run is governed
//! by.
//!
//! Here a rung is named twice: as a variant of [`Layer`], and as the
//! arm of [`Ladder::resolve`] that says which slot of a
//! `kernel::LayeredValue` it fills. Both are exhaustive matches, so a
//! rung added later is a compiler error until it is placed, and every
//! concern picks it up at once.
//!
//! Which rung wins is not decided here: `kernel::LayeredValue::resolve`
//! answers that, and this module only says which rungs there are and
//! what each of them states.

use std::path::{Path, PathBuf};

use kernel::layout::{CONFIG_FILE, CityLayout};
use kernel::{Address, AxCode, AxError, LayeredValue, RESERVED_PREFIX};

use super::{ConfigLayer, refuse};
use crate::building::Building;

/// One rung of the City -> Building -> Resident ladder, from the
/// farthest scope to the nearest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    City,
    Building,
    Resident,
}

impl Layer {
    /// Every rung there is, farthest first. The order the ladder is
    /// read in and the order a nearer value overrides a farther one.
    pub(crate) const ALL: [Layer; 3] = [Layer::City, Layer::Building, Layer::Resident];

    /// Where this rung's file lives for a run at `addr`.
    ///
    /// One expression for every rung: the scope this rung speaks for,
    /// then that scope's own reserved subtree. The city's file is that
    /// rule at the root scope rather than a case of its own, so a rung
    /// added later cannot land somewhere the runs it governs may write.
    ///
    /// # Errors
    /// Propagates the reserved-subtree refusal from [`Building::of`]:
    /// an address with no building has no building rung to read.
    pub(crate) fn file(self, city_root: &Path, addr: &Address) -> Result<PathBuf, AxError> {
        let layout = CityLayout::new(city_root);
        Ok(match self {
            // `CityLayout::config` takes the address of a scope, and
            // the city root is the one scope no address names.
            Layer::City => layout.root().join(RESERVED_PREFIX).join(CONFIG_FILE),
            Layer::Building => layout.config(Building::of(addr)?.addr()),
            Layer::Resident => layout.config(addr),
        })
    }
}

/// Every rung that exists for one address, read, farthest first.
///
/// A value rather than three variables: a caller folds over it and
/// never names a rung, which is what keeps adding one from touching
/// every concern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ladder {
    rungs: Vec<(Layer, ConfigLayer)>,
}

impl Ladder {
    /// Reads every rung that exists for a run at `addr`.
    ///
    /// A rung whose file is one a nearer rung already read is dropped:
    /// an address that *is* its building has two rungs, not three, and
    /// the same file counted twice would suggest it can override
    /// itself.
    ///
    /// # Errors
    /// Refuses an address with no building, a file that cannot be read,
    /// and a file that does not parse. A missing file is not an error:
    /// it is the ordinary case, and it states nothing.
    pub(crate) fn read(city_root: &Path, addr: &Address) -> Result<Ladder, AxError> {
        let mut rungs: Vec<(Layer, ConfigLayer)> = Vec::with_capacity(Layer::ALL.len());
        let mut read: Vec<PathBuf> = Vec::with_capacity(Layer::ALL.len());
        for rung in Layer::ALL {
            let file = rung.file(city_root, addr)?;
            if read.contains(&file) {
                continue;
            }
            rungs.push((rung, stated(&file)?));
            read.push(file);
        }
        Ok(Ladder { rungs })
    }

    /// One concern across the whole ladder, each rung in the slot it
    /// speaks for.
    ///
    /// The only place a rung is matched to a slot. `kernel::freeze`
    /// takes it from here and decides which rung wins and what an
    /// unstated concern falls back to.
    pub(crate) fn resolve<T>(&self, stated: impl Fn(&ConfigLayer) -> Option<T>) -> LayeredValue<T> {
        let mut across = LayeredValue::default();
        for (rung, layer) in &self.rungs {
            let held = stated(layer);
            match rung {
                Layer::City => across.city = held,
                Layer::Building => across.building = held,
                Layer::Resident => across.resident = held,
            }
        }
        across
    }
}

/// What one file states. Absent states nothing, which is how most
/// rungs stay: a value is written where somebody meant to depart from
/// the default.
fn stated(file: &Path) -> Result<ConfigLayer, AxError> {
    match std::fs::read_to_string(file) {
        // Several files can fail; the refusal says which one did.
        Ok(text) => ConfigLayer::parse(&text)
            .map_err(|err| refuse(format!("{}: {}", file.display(), err.subject()))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(ConfigLayer::default()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a configuration layer",
            format!("{}: {err}", file.display()),
        )
        .with_recovery("fix the file's permissions; a configuration that exists is read")),
    }
}
