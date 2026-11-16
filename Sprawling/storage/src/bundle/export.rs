// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Bundles: export, manifest, restore.

use std::path::Path;

use crate::alias::WriteTarget;
use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::files::{
    copy_city_files, copy_tree, count_files, count_records, head_of, only_city_files, walk,
};
use super::history::{self, Carried, History};
use super::landing::{Bits, land};
use super::manifest::{CAS, CITY, LEDGER, MANIFEST, Manifest};

/// One count taken on the city against the same count taken on the
/// bundle.
fn agree(what: &str, source: u64, bundle: u64) -> Result<(), StorageError> {
    if source == bundle {
        return Ok(());
    }
    Err(StorageError::Bundle {
        op: "export",
        detail: format!("the city holds {source} {what}(s) and the bundle holds {bundle}"),
    })
}

/// Export and restore. A namespace rather than a value: neither
/// direction holds state between calls.
pub struct Bundle;

impl Bundle {
    /// Writes a bundle of `city_root` into `dest`.
    ///
    /// # Errors
    /// Propagates read and write failures, naming the path; refuses a
    /// city whose ledger cannot be read, because a bundle of an
    /// unreadable history is a backup of nothing.
    pub fn export(city_root: &Path, dest: &Path) -> Result<Manifest, StorageError> {
        Bundle::export_with(Box::new(RealFs::new()), city_root, dest)
    }

    pub(crate) fn export_with(
        mut vfs: Box<dyn Vfs>,
        city_root: &Path,
        dest: &Path,
    ) -> Result<Manifest, StorageError> {
        let layout = kernel::layout::CityLayout::new(city_root);
        let ledger_dir = layout.ledger();
        let cas_dir = layout.cas();
        let (dest_ledger, dest_cas, dest_city) =
            (dest.join(LEDGER), dest.join(CAS), dest.join(CITY));
        let ledger_files = copy_tree(vfs.as_mut(), dest, &ledger_dir, &dest_ledger)?;
        let cas_copied = copy_tree(vfs.as_mut(), dest, &cas_dir, &dest_cas)?;
        let city_copied = copy_city_files(vfs.as_mut(), dest, city_root, &dest_city)?;
        history::export(vfs.as_mut(), city_root, dest)?;
        // Every number in the manifest is read back from the bundle, so
        // the manifest states what a reader of the bundle will find.
        let manifest = Manifest {
            history: Carried::of(vfs.as_ref(), dest)?,
            ..Manifest::of(vfs.as_ref(), &dest_ledger, &dest_cas, &dest_city)?
        };
        // A manifest read only from the bundle certifies itself: a copy
        // that silently dropped half the city agrees with its own
        // manifest. The source side is therefore counted separately and
        // compared field by field, and a disagreement fails the export
        // rather than producing a backup that is short.
        agree(
            "ledger file",
            ledger_files,
            count_files(vfs.as_ref(), &dest_ledger)?,
        )?;
        agree("cas object", cas_copied, manifest.cas_objects)?;
        agree("city file", city_copied, manifest.files)?;
        agree(
            "ledger record",
            count_records(vfs.as_ref(), &ledger_dir)?,
            manifest.records,
        )?;
        let source_head = head_of(vfs.as_ref(), &ledger_dir)?;
        if source_head != manifest.head {
            return Err(StorageError::Bundle {
                op: "export",
                detail: format!(
                    "the city's history ends {source_head} and the bundle's ends {}",
                    manifest.head
                ),
            });
        }
        let target = WriteTarget::within("write a bundle manifest", dest, &dest.join(MANIFEST))?;
        land(
            vfs.as_mut(),
            target,
            manifest.to_json().as_bytes(),
            Bits::OfReplaced,
        )?;
        Ok(manifest)
    }

    /// Reads a bundle's manifest without restoring it.
    ///
    /// # Errors
    /// Refuses a directory with no readable manifest.
    pub fn read_manifest(bundle: &Path) -> Result<Manifest, StorageError> {
        let vfs = RealFs::new();
        let at = bundle.join(MANIFEST);
        let bytes = vfs
            .read(&at)
            .map_err(io_err("read a bundle manifest", &at))?;
        Manifest::from_json(&bytes, &at)
    }

    /// Restores a bundle into `city_root`, which must not already hold a
    /// ledger.
    ///
    /// Restoring verifies: the chain is walked, and the record count and
    /// head hash are compared against the manifest. A bundle that copied
    /// short is refused here rather than becoming a city that is quietly
    /// missing its last hour.
    ///
    /// # Errors
    /// Refuses an occupied city root, a bundle without a manifest, and
    /// any disagreement between the manifest and what was restored.
    pub fn restore(bundle: &Path, city_root: &Path) -> Result<Manifest, StorageError> {
        Bundle::restore_with(Box::new(RealFs::new()), bundle, city_root)
    }

    pub(crate) fn restore_with(
        mut vfs: Box<dyn Vfs>,
        bundle: &Path,
        city_root: &Path,
    ) -> Result<Manifest, StorageError> {
        let mut claimed = {
            let at = bundle.join(MANIFEST);
            let bytes = vfs
                .read(&at)
                .map_err(io_err("read a bundle manifest", &at))?;
            Manifest::from_json(&bytes, &at)?
        };
        // A bundle carries city files and nothing else: git metadata in
        // one is forged, and restoring it would plant hooks. Refused
        // before anything is copied, all or nothing (`crates/storage/spec/Bundle.lean` §8-12),
        // except the repository a v0.0.6 export carried whole, whose
        // files that manifest counted and which lands as history.
        let repository = only_city_files(vfs.as_ref(), &bundle.join(CITY))?;
        claimed.files = claimed.files.saturating_sub(repository);
        let layout = kernel::layout::CityLayout::new(city_root);
        let ledger_dir = layout.ledger();
        let cas_dir = layout.cas();
        if !walk(vfs.as_ref(), &ledger_dir)?.is_empty() {
            return Err(StorageError::Bundle {
                op: "restore",
                detail: format!("{} already holds a ledger", ledger_dir.display()),
            });
        }
        // Counted off the bundle before anything is copied: a pack that
        // went missing is refused while the city root is still empty.
        let carried = Carried::of(vfs.as_ref(), bundle)?;
        if carried != claimed.history {
            return Err(StorageError::Bundle {
                op: "restore",
                detail: format!(
                    "the bundle claims {} history pack(s) and {} ref(s), and holds {} and {}",
                    claimed.history.packs, claimed.history.refs, carried.packs, carried.refs
                ),
            });
        }
        let history = History::read(vfs.as_ref(), bundle, city_root)?;
        copy_tree(vfs.as_mut(), city_root, &bundle.join(LEDGER), &ledger_dir)?;
        copy_tree(vfs.as_mut(), city_root, &bundle.join(CAS), &cas_dir)?;
        copy_city_files(vfs.as_mut(), city_root, &bundle.join(CITY), city_root)?;
        if let Some(history) = history {
            history.land(city_root)?;
        }

        // All four numbers, because a bundle that lost its objects has
        // the history that points at them: the chain verifies, every
        // Locator resolves to nothing, and only the object count says
        // so.
        let restored = Manifest {
            history: carried,
            ..Manifest::of(vfs.as_ref(), &ledger_dir, &cas_dir, city_root)?
        };
        if restored != claimed {
            return Err(StorageError::Bundle {
                op: "restore",
                detail: format!(
                    "the bundle claims {} record(s) ending {} with {} cas object(s) and {} file(s), \
                     and {} record(s) ending {} with {} cas object(s) and {} file(s) arrived",
                    claimed.records,
                    claimed.head,
                    claimed.cas_objects,
                    claimed.files,
                    restored.records,
                    restored.head,
                    restored.cas_objects,
                    restored.files
                ),
            });
        }
        Ok(restored)
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod history_tests;
