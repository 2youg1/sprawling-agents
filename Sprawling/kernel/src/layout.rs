// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::{Address, RESERVED_PREFIX};

pub const LEDGER_DIR: &str = "ledger";

pub const CAS_DIR: &str = "cas";

pub const SNAPSHOT_DIR: &str = "snapshot";

pub const LIBRARY_DIR: &str = "library";

pub const BUILDING_SHELF: &str = "skills";

pub const SKILL_FILE: &str = "SKILL.md";

pub const SHELF_INDEX: &str = "INDEX.toml";

pub const CONFIG_FILE: &str = "CONFIG.toml";

pub const SESSION_CATALOG_FILE: &str = "CATALOG.toml";

pub const FILTERS_FILE: &str = "FILTERS.toml";

pub const ARCHIVE_DIR: &str = "Archive";

pub const JOB_FILE: &str = "JOB.md";

pub const HANDOFF_FILE: &str = "Handoff.md";

pub const URBANITE_FILE: &str = "URBANITE.md";

pub const GUIDE_FILE: &str = "GUIDE.toml";

pub const TRANSCRIPT_EXT: &str = "jsonl";

pub const RUN_ID_PATTERN: &str = "????????-????-????-????-????????????";

pub const REMOTE_DIR: &str = "remote";

pub const DEVICES_FILE: &str = "devices.toml";

pub const BROWSERS_FILE: &str = "browsers.toml";

pub const PLAYBACK_DIR: &str = "playback";

pub const MONITOR_FILE: &str = "MONITOR.toml";

#[must_use]
pub fn document_staging_name(name: &OsStr) -> OsString {
    let mut staged = OsString::from(STAGING_PREFIX);
    staged.push(name);
    staged.push(STAGING_SUFFIX);
    staged
}

#[must_use]
pub fn is_document_staging_name(name: &OsStr) -> bool {
    let spelled = name.as_encoded_bytes();
    spelled.len() > STAGING_PREFIX.len().saturating_add(STAGING_SUFFIX.len())
        && spelled.starts_with(STAGING_PREFIX.as_bytes())
        && spelled.ends_with(STAGING_SUFFIX.as_bytes())
}

const STAGING_PREFIX: &str = ".";
const STAGING_SUFFIX: &str = ".staging";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CityLayout {
    root: PathBuf,
}

impl CityLayout {

    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn scope(&self, addr: &Address) -> PathBuf {
        let mut path = self.root.clone();
        for segment in addr.as_str().split('/') {
            path.push(segment);
        }
        path
    }

    #[must_use]
    pub fn ledger(&self) -> PathBuf {
        self.governed_root().join(LEDGER_DIR)
    }

    #[must_use]
    pub fn snapshot(&self) -> PathBuf {
        self.governed_root().join(SNAPSHOT_DIR)
    }

    #[must_use]
    pub fn cas(&self) -> PathBuf {
        self.governed_root().join(CAS_DIR)
    }

    #[must_use]
    pub fn library(&self) -> PathBuf {
        self.governed_root().join(LIBRARY_DIR)
    }

    #[must_use]
    pub fn config(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(CONFIG_FILE)
    }

    pub fn session_catalog(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(SESSION_CATALOG_FILE)
    }

    #[must_use]
    pub fn city_config(&self) -> PathBuf {
        self.governed_root().join(CONFIG_FILE)
    }

    #[must_use]
    pub fn city_filters(&self) -> PathBuf {
        self.governed_root().join(FILTERS_FILE)
    }

    #[must_use]
    pub fn building_skills(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(BUILDING_SHELF)
    }

    #[must_use]
    pub fn filters(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(FILTERS_FILE)
    }

    #[must_use]
    pub fn archive(&self, building: &Address) -> PathBuf {
        self.scope(building).join(ARCHIVE_DIR)
    }

    #[must_use]
    pub fn job(&self, addr: &Address) -> PathBuf {
        self.scope(addr).join(JOB_FILE)
    }

    #[must_use]
    pub fn handoff(&self, room: &Address) -> PathBuf {
        self.scope(room).join(HANDOFF_FILE)
    }

    #[must_use]
    pub fn urbanite(&self, addr: &Address) -> PathBuf {
        self.scope(addr).join(URBANITE_FILE)
    }

    #[must_use]
    pub fn guide(&self) -> PathBuf {
        self.governed_root().join(GUIDE_FILE)
    }

    #[must_use]
    pub fn devices(&self) -> PathBuf {
        self.governed_root().join(REMOTE_DIR).join(DEVICES_FILE)
    }

    #[must_use]
    pub fn browsers(&self) -> PathBuf {
        self.governed_root().join(BROWSERS_FILE)
    }

    #[must_use]
    pub fn playback_exports(&self) -> PathBuf {
        self.governed_root().join(PLAYBACK_DIR)
    }

    #[must_use]
    pub fn monitor(&self) -> PathBuf {
        self.governed_root().join(MONITOR_FILE)
    }

    #[must_use]
    pub fn of_ledger(dir: &Path) -> Option<CityLayout> {
        if dir.file_name()? != LEDGER_DIR {
            return None;
        }
        let governed = dir.parent()?;
        if governed.file_name()? != RESERVED_PREFIX {
            return None;
        }
        let root = governed.parent()?;
        if root.as_os_str().is_empty() {
            return None;
        }
        Some(CityLayout::new(root))
    }

    #[must_use]
    pub fn city_address(&self) -> Option<Address> {
        self.root
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Address::parse(name).ok())
    }

    fn governed_root(&self) -> PathBuf {
        self.root.join(RESERVED_PREFIX)
    }

    fn governed(&self, addr: &Address) -> PathBuf {
        self.scope(addr).join(RESERVED_PREFIX)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
