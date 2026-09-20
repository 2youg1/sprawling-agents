// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Bundles: export, manifest, restore.

use std::path::Path;

use crate::error::{MemoryError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::files::{
    copy_city_files, copy_tree, count_files, count_records, head_of, walk, write_file,
};
use super::manifest::{CAS, CITY, LEDGER, MANIFEST, Manifest, RESERVED};

/// One count taken on the city against the same count taken on the
/// bundle.
fn agree(what: &str, source: u64, bundle: u64) -> Result<(), MemoryError> {
    if source == bundle {
        return Ok(());
    }
    Err(MemoryError::Bundle {
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
    pub fn export(city_root: &Path, dest: &Path) -> Result<Manifest, MemoryError> {
        Bundle::export_with(Box::new(RealFs::new()), city_root, dest)
    }

    pub(crate) fn export_with(
        mut vfs: Box<dyn Vfs>,
        city_root: &Path,
        dest: &Path,
    ) -> Result<Manifest, MemoryError> {
        let ledger_dir = city_root.join(RESERVED).join(LEDGER);
        let cas_dir = city_root.join(RESERVED).join(CAS);
        let (dest_ledger, dest_cas, dest_city) =
            (dest.join(LEDGER), dest.join(CAS), dest.join(CITY));
        let ledger_files = copy_tree(vfs.as_mut(), &ledger_dir, &dest_ledger)?;
        let cas_copied = copy_tree(vfs.as_mut(), &cas_dir, &dest_cas)?;
        let city_copied = copy_city_files(vfs.as_mut(), city_root, &dest_city)?;
        // Every number in the manifest is read back from the bundle, so
        // the manifest states what a reader of the bundle will find.
        let manifest = Manifest {
            records: count_records(vfs.as_ref(), &dest_ledger)?,
            head: head_of(vfs.as_ref(), &dest_ledger)?,
            cas_objects: count_files(vfs.as_ref(), &dest_cas)?,
            files: count_files(vfs.as_ref(), &dest_city)?,
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
            return Err(MemoryError::Bundle {
                op: "export",
                detail: format!(
                    "the city's history ends {source_head} and the bundle's ends {}",
                    manifest.head
                ),
            });
        }
        write_file(
            vfs.as_mut(),
            &dest.join(MANIFEST),
            manifest.to_json().as_bytes(),
        )?;
        Ok(manifest)
    }

    /// Reads a bundle's manifest without restoring it.
    ///
    /// # Errors
    /// Refuses a directory with no readable manifest.
    pub fn read_manifest(bundle: &Path) -> Result<Manifest, MemoryError> {
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
    pub fn restore(bundle: &Path, city_root: &Path) -> Result<Manifest, MemoryError> {
        Bundle::restore_with(Box::new(RealFs::new()), bundle, city_root)
    }

    pub(crate) fn restore_with(
        mut vfs: Box<dyn Vfs>,
        bundle: &Path,
        city_root: &Path,
    ) -> Result<Manifest, MemoryError> {
        let claimed = {
            let at = bundle.join(MANIFEST);
            let bytes = vfs
                .read(&at)
                .map_err(io_err("read a bundle manifest", &at))?;
            Manifest::from_json(&bytes, &at)?
        };
        let ledger_dir = city_root.join(RESERVED).join(LEDGER);
        if !walk(vfs.as_ref(), &ledger_dir).is_empty() {
            return Err(MemoryError::Bundle {
                op: "restore",
                detail: format!("{} already holds a ledger", ledger_dir.display()),
            });
        }
        copy_tree(vfs.as_mut(), &bundle.join(LEDGER), &ledger_dir)?;
        copy_tree(
            vfs.as_mut(),
            &bundle.join(CAS),
            &city_root.join(RESERVED).join(CAS),
        )?;
        copy_tree(vfs.as_mut(), &bundle.join(CITY), city_root)?;

        let restored = Manifest {
            records: count_records(vfs.as_ref(), &ledger_dir)?,
            head: head_of(vfs.as_ref(), &ledger_dir)?,
            cas_objects: count_files(vfs.as_ref(), &city_root.join(RESERVED).join(CAS))?,
            files: count_files(vfs.as_ref(), city_root)?,
        };
        if restored.records != claimed.records || restored.head != claimed.head {
            return Err(MemoryError::Bundle {
                op: "restore",
                detail: format!(
                    "the bundle claims {} record(s) ending {}, and {} record(s) ending {} arrived",
                    claimed.records, claimed.head, restored.records, restored.head
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
mod tests {
    use super::super::files::open_restored;
    use super::super::fixture::city_with;
    use super::*;
    use kernel::{GENESIS_PREV, TimeMs};

    #[test]
    fn a_city_comes_back_in_an_empty_directory_and_its_chain_verifies() {
        let home = tempfile::tempdir().unwrap();
        city_with(3, home.path());
        let carried = tempfile::tempdir().unwrap();
        let exported = Bundle::export(home.path(), carried.path()).unwrap();
        assert_eq!(exported.records(), 3);
        assert_ne!(exported.head(), GENESIS_PREV.to_string());

        let elsewhere = tempfile::tempdir().unwrap();
        let restored = Bundle::restore(carried.path(), elsewhere.path()).unwrap();
        assert_eq!(restored, exported);
        // The work came with it, not only the history.
        assert!(elsewhere.path().join("City.md").exists());
        assert!(elsewhere.path().join("lab").join("Roadmap.md").exists());
        // And the restored city is one a writer can continue.
        open_restored(elsewhere.path(), TimeMs::new(9)).unwrap();
    }

    /// A filesystem that accepts one write and does not keep it, which
    /// is what a full disk and a cancelled copy look like from here.
    struct LosesRoadmap(RealFs);

    impl LosesRoadmap {
        fn swallowed(path: &std::path::Path) -> bool {
            path.ends_with("Roadmap.md")
        }
    }

    impl crate::vfs::Vfs for LosesRoadmap {
        fn create_dir_all(&mut self, dir: &std::path::Path) -> std::io::Result<()> {
            self.0.create_dir_all(dir)
        }
        fn list(&self, dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
            self.0.list(dir)
        }
        fn list_dirs(&self, dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
            self.0.list_dirs(dir)
        }
        fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
            self.0.read(path)
        }
        fn read_at(
            &self,
            path: &std::path::Path,
            offset: u64,
            len: u64,
        ) -> std::io::Result<Vec<u8>> {
            self.0.read_at(path, offset, len)
        }
        fn append(&mut self, path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
            if Self::swallowed(path) {
                return Ok(());
            }
            self.0.append(path, bytes)
        }
        fn truncate(&mut self, path: &std::path::Path, len: u64) -> std::io::Result<()> {
            self.0.truncate(path, len)
        }
        fn sync_data(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            if Self::swallowed(path) {
                return Ok(());
            }
            self.0.sync_data(path)
        }
        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
            self.0.rename(from, to)
        }
        fn sync_dir(&mut self, dir: &std::path::Path) -> std::io::Result<()> {
            self.0.sync_dir(dir)
        }
        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.0.remove_file(path)
        }
        fn exists(&self, path: &std::path::Path) -> bool {
            self.0.exists(path)
        }
    }

    /// The manifest is read back from the bundle, so on its own it
    /// cannot tell a complete bundle from a short one. This is the case
    /// that the source-side counts exist for.
    #[test]
    fn a_city_file_the_bundle_never_received_fails_the_export() {
        let home = tempfile::tempdir().unwrap();
        city_with(2, home.path());
        let carried = tempfile::tempdir().unwrap();
        let err = Bundle::export_with(
            Box::new(LosesRoadmap(RealFs::new())),
            home.path(),
            carried.path(),
        )
        .unwrap_err();
        let ax = err.into_ax();
        assert!(
            ax.subject().contains("city file"),
            "the export has to name the count that disagreed: {}",
            ax.subject()
        );
    }

    #[test]
    fn a_short_copy_is_refused_rather_than_restored_quietly() {
        let home = tempfile::tempdir().unwrap();
        city_with(4, home.path());
        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();

        // Lose the last record, as an interrupted copy would.
        let segment = std::fs::read_dir(carried.path().join(LEDGER))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
            .unwrap();
        let bytes = std::fs::read(&segment).unwrap();
        let cut = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .and_then(|last| bytes[..last].iter().rposition(|byte| *byte == b'\n'))
            .unwrap();
        std::fs::write(&segment, &bytes[..=cut]).unwrap();

        let elsewhere = tempfile::tempdir().unwrap();
        let err = Bundle::restore(carried.path(), elsewhere.path()).unwrap_err();
        let ax = err.into_ax();
        assert!(
            ax.subject().contains("record(s) ending"),
            "a partial copy is not a city: {}",
            ax.subject()
        );
    }

    #[test]
    fn a_city_is_never_restored_on_top_of_another() {
        let home = tempfile::tempdir().unwrap();
        city_with(2, home.path());
        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();

        // The city it came from still has its ledger, so restoring back
        // onto it would be a merge of two histories.
        let err = Bundle::restore(carried.path(), home.path()).unwrap_err();
        assert!(err.into_ax().subject().contains("already holds a ledger"));
    }
}
