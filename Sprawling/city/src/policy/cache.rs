// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use kernel::{Address, AxError};

use super::{BuildingRules, load, rules_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: SystemTime,
    len: u64,
}

#[derive(Debug)]
pub struct RulesCache {
    city_root: PathBuf,
    kept: Mutex<BTreeMap<Address, (Stamp, Arc<BuildingRules>)>>,
}

impl RulesCache {

    pub fn new(city_root: &Path) -> RulesCache {
        RulesCache {
            city_root: city_root.to_path_buf(),
            kept: Mutex::new(BTreeMap::new()),
        }
    }

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
