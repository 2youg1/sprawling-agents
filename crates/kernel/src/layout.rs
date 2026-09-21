// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a city keeps each kind of file on disk: one derivation from
//! the city root and an address (kernel-SPEC.md section 8-56).
//!
//! Every directory name and file name the city writes is declared here
//! once, and every path it writes them to is spelled here once. Before
//! this module each caller joined its own segments, so the reserved
//! subtree had two names, the object store had six spellings, and a
//! rename could reach five of them and leave the sixth reading an empty
//! directory.
//!
//! Nothing here touches a disk. A `CityLayout` is a value: it answers
//! where a file would be, and the caller that owns the I/O decides
//! whether to create it, read it, or refuse. That keeps the layout
//! inside `kernel`, where the address grammar it rests on already is.
//!
//! One rule governs the placement itself: **what governs a scope lives
//! in that scope's reserved subtree, and no write domain reaches it**
//! (`Address::is_reserved`). The rules of a building, its configuration
//! layer and its filters are therefore under [`RESERVED_PREFIX`], while
//! the documents residents write — the job, the handoff, the urbanite
//! page, the archive — sit in the open where residents can edit them.

use std::path::{Path, PathBuf};

use crate::{Address, RESERVED_PREFIX};

/// The append-only history of a city, under the city's reserved subtree.
pub const LEDGER_DIR: &str = "ledger";
/// The content-addressed object store, under the city's reserved subtree.
pub const CAS_DIR: &str = "cas";
/// The city's shelves of skills, under the city's reserved subtree.
pub const LIBRARY_DIR: &str = "library";
/// One building's projection of the sessions run in it, under that
/// building's reserved subtree. Derived from the Ledger, so a person
/// reading a building finds its history beside it; disposable, so nothing
/// in the product reads it.
pub const SESSIONS_DIR: &str = "sessions";
/// A building's own shelf of skills, under that building's reserved
/// subtree: what this building knows and no other building is given.
pub const BUILDING_SHELF: &str = "skills";
/// What one configuration layer declares. The same file name at every
/// layer, because the layer is the scope it sits in rather than a name.
pub const CONFIG_FILE: &str = "CONFIG.toml";
/// What a scope keeps out of the transcript it sends to a model.
pub const FILTERS_FILE: &str = "FILTERS.toml";
/// Where a building files work it has finished with, by kind and day.
pub const ARCHIVE_DIR: &str = "Archive";
/// The task of one session, in the room it is run from.
pub const JOB_FILE: &str = "JOB.md";
/// What the next agent needs before it starts, per room.
pub const HANDOFF_FILE: &str = "Handoff.md";
/// Who a standing resident is, in that resident's own directory.
pub const URBANITE_FILE: &str = "URBANITE.md";

/// The disk layout of one city, derived from its root.
///
/// Construct it once per city root and ask it for paths; the segments
/// are never joined by hand at a call site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CityLayout {
    root: PathBuf,
}

impl CityLayout {
    /// The layout of the city rooted at `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// The city root this layout derives from.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory an address names: the city root with the address's
    /// segments below it.
    ///
    /// Segment by segment rather than one joined string, because a
    /// city served on Windows and a city served on Linux must lay the
    /// same address down as the same directory.
    #[must_use]
    pub fn scope(&self, addr: &Address) -> PathBuf {
        let mut path = self.root.clone();
        for segment in addr.as_str().split('/') {
            path.push(segment);
        }
        path
    }

    /// The whole city's history.
    #[must_use]
    pub fn ledger(&self) -> PathBuf {
        self.governed_root().join(LEDGER_DIR)
    }

    /// The whole city's object store.
    #[must_use]
    pub fn cas(&self) -> PathBuf {
        self.governed_root().join(CAS_DIR)
    }

    /// The city's shelves of skills.
    #[must_use]
    pub fn library(&self) -> PathBuf {
        self.governed_root().join(LIBRARY_DIR)
    }

    /// The configuration layer a scope declares.
    #[must_use]
    pub fn config(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(CONFIG_FILE)
    }

    /// The configuration layer the city itself declares.
    ///
    /// The same rule as [`config`](Self::config) at the root, and a
    /// method of its own because the root is not an address: a caller
    /// without one used to reach past this type and join the two names
    /// by hand, which is how the reserved prefix grew its second home.
    #[must_use]
    pub fn city_config(&self) -> PathBuf {
        self.governed_root().join(CONFIG_FILE)
    }

    /// What the city as a whole keeps out of what it sends a model.
    #[must_use]
    pub fn city_filters(&self) -> PathBuf {
        self.governed_root().join(FILTERS_FILE)
    }

    /// A building's own shelf of skills.
    #[must_use]
    pub fn building_skills(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(BUILDING_SHELF)
    }

    /// What a scope keeps out of what it sends a model.
    #[must_use]
    pub fn filters(&self, addr: &Address) -> PathBuf {
        self.governed(addr).join(FILTERS_FILE)
    }

    /// Where a building files finished work.
    ///
    /// In the open rather than under the reserved subtree: residents
    /// write these documents and read them back.
    #[must_use]
    pub fn archive(&self, building: &Address) -> PathBuf {
        self.scope(building).join(ARCHIVE_DIR)
    }

    /// The task written for the session run at this address.
    #[must_use]
    pub fn job(&self, addr: &Address) -> PathBuf {
        self.scope(addr).join(JOB_FILE)
    }

    /// What one room hands to whoever picks it up next.
    ///
    /// The room's rather than the building's: two sessions of one
    /// building can freeze at once, and one file for both would be two
    /// contents fighting over one name.
    #[must_use]
    pub fn handoff(&self, room: &Address) -> PathBuf {
        self.scope(room).join(HANDOFF_FILE)
    }

    /// Who the standing resident at this address is.
    #[must_use]
    pub fn urbanite(&self, addr: &Address) -> PathBuf {
        self.scope(addr).join(URBANITE_FILE)
    }

    /// The projection of one session's records: the room address is the
    /// session's identity, and a building is its first segment.
    ///
    /// An address below a building becomes one file under that
    /// building's sessions, its name the rooms below the building, so the
    /// nesting of rooms is the nesting of the files and no reader has to
    /// know which segment was a building. A run that named no session
    /// works at the building's own address, and its file then carries the
    /// building's name.
    #[must_use]
    pub fn session_slice(&self, room: &Address) -> PathBuf {
        let raw = room.as_str();
        let (building, under) = raw.split_once('/').unwrap_or((raw, ""));
        let name = if under.is_empty() { building } else { under };
        let mut path = self.root.clone();
        path.push(building);
        path.push(RESERVED_PREFIX);
        path.push(SESSIONS_DIR);
        path.push(format!("{name}.jsonl"));
        path
    }

    /// Whether `relative` names a session slice: a file under some
    /// scope's reserved `sessions` directory.
    ///
    /// A slice is a disposable projection of the Ledger
    /// (memory-SPEC 8-24) and the city appends to it while a wave runs,
    /// so the checkpoint never stages one: git reads a workdir file it
    /// believes it knows, and a file the city itself keeps writing makes
    /// that read refuse the whole wave. The reserved prefix is required
    /// immediately before `sessions`, so a person's own directory that
    /// happens to carry that name is not mistaken for the projection.
    /// One predicate rather than each caller spelling the directory,
    /// which is what `xtask slices` holds.
    #[must_use]
    pub fn is_session_projection(relative: &Path) -> bool {
        let mut held = false;
        for component in relative.components() {
            let Some(name) = component.as_os_str().to_str() else {
                held = false;
                continue;
            };
            if name.eq_ignore_ascii_case(SESSIONS_DIR) {
                return held;
            }
            held = name.eq_ignore_ascii_case(RESERVED_PREFIX);
        }
        false
    }

    /// The layout whose ledger is `dir`, when `dir` is a city's.
    ///
    /// The inverse of [`ledger`](Self::ledger), for the writer that is
    /// handed the ledger directory and must find the root every other
    /// path is derived from. `None` for a directory that is not a ledger
    /// under a reserved prefix: a fixture, a bundle being verified, or a
    /// store somebody opened directly, none of which is a city.
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

    /// The city's own name, read from the directory it lives in.
    ///
    /// The one address that names the city rather than a room: the
    /// genesis record carries it so a city can say who it is, and the
    /// session projection must file it nowhere, because no building is
    /// named after the city inside itself. Not every directory name is
    /// an address - a path can hold characters an address may not - and
    /// a city whose directory cannot be spelled as an address simply has
    /// no name to show, which is honest and rare.
    #[must_use]
    pub fn city_address(&self) -> Option<Address> {
        self.root
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Address::parse(name).ok())
    }

    /// The city root's own reserved subtree.
    fn governed_root(&self) -> PathBuf {
        self.root.join(RESERVED_PREFIX)
    }

    /// A scope's own reserved subtree: what governs that scope, where no
    /// write domain reaches it.
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
