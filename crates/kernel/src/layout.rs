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
mod tests {
    use super::*;

    fn layout() -> CityLayout {
        CityLayout::new(Path::new("/city"))
    }

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    #[test]
    fn a_nested_address_becomes_one_directory_per_segment() {
        let expected = Path::new("/city").join("lab").join("refactor");
        assert_eq!(layout().scope(&addr("lab/refactor")), expected);
    }

    #[test]
    fn the_city_wide_stores_sit_under_the_city_s_reserved_subtree() {
        let governed = Path::new("/city").join(RESERVED_PREFIX);
        assert_eq!(layout().ledger(), governed.join(LEDGER_DIR));
        assert_eq!(layout().cas(), governed.join(CAS_DIR));
        assert_eq!(layout().library(), governed.join(LIBRARY_DIR));
    }

    #[test]
    fn what_governs_a_scope_is_unreachable_by_any_write_domain() {
        let building = addr("lab");
        let governing = [
            layout().config(&building),
            layout().filters(&building),
            layout().building_skills(&building),
        ];
        for path in governing {
            let spelled = path.to_string_lossy().replace('\\', "/");
            let as_address = spelled.trim_start_matches("/city/").to_owned();
            assert!(
                addr(&as_address).is_reserved(),
                "{spelled} is not in a reserved subtree"
            );
        }
    }

    #[test]
    fn what_residents_write_stays_where_residents_can_reach_it() {
        let room = addr("lab/refactor");
        let open = [
            layout().job(&room),
            layout().handoff(&room),
            layout().urbanite(&room),
            layout().archive(&addr("lab")),
        ];
        for path in open {
            let spelled = path.to_string_lossy().replace('\\', "/");
            assert!(
                !spelled.contains(RESERVED_PREFIX),
                "{spelled} is governing rather than written"
            );
        }
    }

    #[test]
    fn a_configuration_layer_is_named_by_the_scope_it_sits_in() {
        assert_eq!(
            layout().config(&addr("lab")),
            Path::new("/city")
                .join("lab")
                .join(RESERVED_PREFIX)
                .join(CONFIG_FILE)
        );
    }
}
