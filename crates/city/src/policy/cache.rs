// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rules a run has already read, kept while the file's stamp holds
//! (city-SPEC 8-2, 12.3).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use kernel::{Address, AxError};

use super::{BuildingRules, load, rules_path};

/// What identifies one version of a `RULES.toml` without reading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: SystemTime,
    len: u64,
}

/// Building rules kept for one run, keyed by the `RULES.toml` stamp.
#[derive(Debug)]
pub struct RulesCache {
    city_root: PathBuf,
    kept: Mutex<BTreeMap<Address, (Stamp, Arc<BuildingRules>)>>,
}

impl RulesCache {
    /// An empty cache over this city.
    pub fn new(city_root: &Path) -> RulesCache {
        RulesCache {
            city_root: city_root.to_path_buf(),
            kept: Mutex::new(BTreeMap::new()),
        }
    }

    /// A building's rules: the kept ones while the file's modification
    /// time and length are unchanged, a fresh [`load`] otherwise.
    ///
    /// An absent or unstattable file is never kept, so the answers for
    /// "no rules" and for a superseded document stay [`load`]'s own.
    ///
    /// # Errors
    /// Whatever [`load`] returns; a failure is never kept.
    pub fn load(&self, addr: &Address) -> Result<Arc<BuildingRules>, AxError> {
        let Some(stamp) = self.stamp(addr) else {
            return load(&self.city_root, addr).map(Arc::new);
        };
        let Ok(mut kept) = self.kept.lock() else {
            return load(&self.city_root, addr).map(Arc::new);
        };
        match kept.get(addr) {
            Some((held, rules)) if *held == stamp => Ok(Arc::clone(rules)),
            Some(_) | None => {
                let rules = Arc::new(load(&self.city_root, addr)?);
                kept.insert(addr.clone(), (stamp, Arc::clone(&rules)));
                Ok(rules)
            }
        }
    }

    /// The file's stamp, or `None` when there is nothing to key on and
    /// [`load`] has to give the answer.
    fn stamp(&self, addr: &Address) -> Option<Stamp> {
        let meta = std::fs::metadata(rules_path(&self.city_root, addr)).ok()?;
        Some(Stamp {
            modified: meta.modified().ok()?,
            len: meta.len(),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code; the reading samples the clock it measures"
)]
mod tests;
