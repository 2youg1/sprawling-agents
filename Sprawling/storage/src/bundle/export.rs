// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

fn agree(what: &str, source: u64, bundle: u64) -> Result<(), StorageError> {
    if source == bundle {
        return Ok(());
    }
    Err(StorageError::Bundle {
        op: "export",
        detail: format!("the city holds {source} {what}(s) and the bundle holds {bundle}"),
    })
}

pub struct Bundle;

impl Bundle {

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

        let manifest = Manifest {
            history: Carried::of(vfs.as_ref(), dest)?,
            ..Manifest::of(vfs.as_ref(), &dest_ledger, &dest_cas, &dest_city)?
        };

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

    pub fn read_manifest(bundle: &Path) -> Result<Manifest, StorageError> {
        let vfs = RealFs::new();
        let at = bundle.join(MANIFEST);
        let bytes = vfs
            .read(&at)
            .map_err(io_err("read a bundle manifest", &at))?;
        Manifest::from_json(&bytes, &at)
    }

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
