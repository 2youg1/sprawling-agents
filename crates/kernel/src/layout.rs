// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a city keeps each kind of file on disk: one derivation from
//! the city root and an address (`crates/kernel/spec/Layout.lean` §8-56).
//!
//! Every directory name and file name the city writes is declared here
//! once, and every path it writes them to is spelled here once, so a
//! rename reaches every caller and none is left reading an empty
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

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::{Address, RESERVED_PREFIX};

/// The append-only history of a city, under the city's reserved subtree.
pub const LEDGER_DIR: &str = "ledger";
/// The content-addressed object store, under the city's reserved subtree.
pub const CAS_DIR: &str = "cas";
/// What the views held after one ledger line, under the city's reserved
/// subtree; a projection a start may discard.
pub const SNAPSHOT_DIR: &str = "snapshot";
/// The city's shelves of skills, under the city's reserved subtree.
pub const LIBRARY_DIR: &str = "library";
/// A building's own shelf of skills, under that building's reserved
/// subtree: what this building knows and no other building is given.
pub const BUILDING_SHELF: &str = "skills";
/// The one document of a skill filed as a directory: a package on a
/// section shelf, and every skill on a shelf the city mounts from
/// elsewhere. It is the layout pi, claude and agents all use; the
/// catalog lists this file's first line, and `read` opens the files
/// beside it by `<name>/<path>`.
pub const SKILL_FILE: &str = "SKILL.md";
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
/// How far the person has got through the first-run guide, under the
/// city's reserved subtree.
pub const GUIDE_FILE: &str = "GUIDE.toml";
/// The extension of what one run saw, written in its room as
/// `<run>.jsonl`.
pub const TRANSCRIPT_EXT: &str = "jsonl";
/// A run id as a git ignore pattern: one `?` for each character of the
/// hyphenated form a [`crate::RunId`] displays, so a building can keep
/// every run's transcript out of its history by name.
pub const RUN_ID_PATTERN: &str = "????????-????-????-????-????????????";
/// What the remote gate keeps on disk, under the city's reserved subtree.
pub const REMOTE_DIR: &str = "remote";
/// The devices paired to reach the city from outside the machine.
pub const DEVICES_FILE: &str = "devices.toml";
/// Where residents' playback exports land, under the city's reserved
/// subtree, one directory per building.
pub const PLAYBACK_DIR: &str = "playback";
/// The beat the performance monitor samples at, under the city's
/// reserved subtree (`crates/sprawling/spec/Monitor.lean` D48).
pub const MONITOR_FILE: &str = "MONITOR.toml";

/// What the staging file of a document named `name` is called: the
/// name hidden by a dot and suffixed `.staging`, in the document's own
/// directory (`crates/kernel/spec/Layout.lean` §8-56).
///
/// The whole-or-nothing document writer stages here and renames over
/// the document; the checkpoint scan refuses this name before git opens
/// it, because the file can be renamed away between the scan's stat and
/// its read (storage D30). Both ask this function, so the two spellings
/// cannot drift apart.
#[must_use]
pub fn document_staging_name(name: &OsStr) -> OsString {
    let mut staged = OsString::from(STAGING_PREFIX);
    staged.push(name);
    staged.push(STAGING_SUFFIX);
    staged
}

/// Whether `name` is spelled the way [`document_staging_name`] spells
/// the staging file of some document.
#[must_use]
pub fn is_document_staging_name(name: &OsStr) -> bool {
    let spelled = name.as_encoded_bytes();
    spelled.len() > STAGING_PREFIX.len().saturating_add(STAGING_SUFFIX.len())
        && spelled.starts_with(STAGING_PREFIX.as_bytes())
        && spelled.ends_with(STAGING_SUFFIX.as_bytes())
}

const STAGING_PREFIX: &str = ".";
const STAGING_SUFFIX: &str = ".staging";

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

    /// The snapshot a start resumes the views from.
    #[must_use]
    pub fn snapshot(&self) -> PathBuf {
        self.governed_root().join(SNAPSHOT_DIR)
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
    /// method of its own because the root is not an address, and a
    /// caller without one would otherwise join the two names by hand.
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

    /// How far the person has got through this city's first-run guide
    /// (`crates/wire/Spec.lean` §8-68).
    ///
    /// Under the city's reserved subtree, because it decides what this
    /// city shows a person when it opens, and no write domain reaches it.
    #[must_use]
    pub fn guide(&self) -> PathBuf {
        self.governed_root().join(GUIDE_FILE)
    }

    /// The devices paired to reach this city from outside the machine.
    ///
    /// Under the city's reserved subtree, because the table decides who
    /// may reach the city and no write domain may change that
    /// (`crates/kernel/spec/Layout.lean` §8-76).
    #[must_use]
    pub fn devices(&self) -> PathBuf {
        self.governed_root().join(REMOTE_DIR).join(DEVICES_FILE)
    }

    /// Where residents' playback exports land, one directory per
    /// building (`crates/accounting/Spec.lean` §8-13, `crates/sprawling/Spec.lean` §8-132).
    ///
    /// Under the city's reserved subtree, because no write domain reaches
    /// it, so a written report is changed only by exporting another; the
    /// city root's `.gitignore` keeps it out of history; and no worktree
    /// holds it, so sweeping one loses no report.
    #[must_use]
    pub fn playback_exports(&self) -> PathBuf {
        self.governed_root().join(PLAYBACK_DIR)
    }

    /// Where this city remembers its monitor's sampling beat: a setting
    /// of how this machine measures, which no write domain reaches.
    #[must_use]
    pub fn monitor(&self) -> PathBuf {
        self.governed_root().join(MONITOR_FILE)
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
