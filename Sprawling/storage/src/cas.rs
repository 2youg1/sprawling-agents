// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod origin;
mod ranges;

pub use origin::BlockOrigin;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use kernel::{B3Hash, Range};

use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

pub struct Cas {
    vfs: Box<dyn Vfs>,
    dir: PathBuf,
}

impl Cas {

    pub fn open(dir: &Path) -> Result<Cas, StorageError> {
        Cas::open_with(Box::new(RealFs::new()), dir)
    }

    pub(crate) fn open_with(mut vfs: Box<dyn Vfs>, dir: &Path) -> Result<Cas, StorageError> {
        let objects = dir.join("b3");
        let tmp = dir.join("tmp");
        vfs.create_dir_all(&objects)
            .map_err(io_err("create cas dir", &objects))?;
        vfs.create_dir_all(&tmp)
            .map_err(io_err("create cas tmp dir", &tmp))?;

        let leftovers = vfs.list(&tmp).map_err(io_err("list cas tmp", &tmp))?;
        for leftover in leftovers
            .into_iter()
            .filter(|leftover| !TmpName::written_here(leftover))
        {
            vfs.remove_file(&leftover)
                .map_err(io_err("sweep cas tmp", &leftover))?;
        }
        Ok(Cas {
            vfs,
            dir: dir.to_path_buf(),
        })
    }

    fn object_path(&self, hash: &B3Hash) -> (PathBuf, PathBuf) {
        let hex = hash.to_string();
        let shard = hex.get(..2).unwrap_or("00");
        let shard_dir = self.dir.join("b3").join(shard);
        let path = shard_dir.join(&hex);
        (shard_dir, path)
    }

    pub fn put(&mut self, bytes: &[u8]) -> Result<B3Hash, StorageError> {
        self.write(bytes, false)
    }

    fn write(&mut self, bytes: &[u8], repair: bool) -> Result<B3Hash, StorageError> {
        let hash = B3Hash::from_bytes(*blake3::hash(bytes).as_bytes());
        let (shard_dir, path) = self.object_path(&hash);
        if self.vfs.exists(&path) {
            if !repair {
                return Ok(hash);
            }
            match self.get(&hash) {
                Ok(_) => return Ok(hash),
                Err(StorageError::CasCorrupt { .. }) => {}
                Err(stopped) => return Err(stopped),
            }
        }
        self.vfs
            .create_dir_all(&shard_dir)
            .map_err(io_err("create cas shard", &shard_dir))?;
        let tmp = self.dir.join("tmp").join(TmpName::next(&hash).0);
        if self.vfs.exists(&tmp) {
            self.vfs
                .truncate(&tmp, 0)
                .map_err(io_err("reset cas tmp", &tmp))?;
        }
        self.vfs
            .append(&tmp, bytes)
            .map_err(io_err("write cas tmp", &tmp))?;
        self.vfs
            .sync_data(&tmp)
            .map_err(io_err("sync cas tmp", &tmp))?;
        self.vfs
            .rename(&tmp, &path)
            .map_err(io_err("rename cas object", &path))?;
        self.vfs
            .make_readonly(&path)
            .map_err(io_err("make cas object read-only", &path))?;
        self.vfs
            .sync_dir(&shard_dir)
            .map_err(io_err("sync cas shard", &shard_dir))?;
        Ok(hash)
    }

    pub fn put_repair(&mut self, bytes: &[u8]) -> Result<B3Hash, StorageError> {
        self.write(bytes, true)
    }

    pub fn contains(&self, hash: &B3Hash) -> bool {
        let (_, path) = self.object_path(hash);
        self.vfs.exists(&path)
    }

    pub fn get(&self, hash: &B3Hash) -> Result<Vec<u8>, StorageError> {
        let (_, path) = self.object_path(hash);
        if !self.vfs.exists(&path) {
            return Err(StorageError::CasMissing {
                hash: hash.to_string(),
            });
        }
        let bytes = self
            .vfs
            .read(&path)
            .map_err(io_err("read cas object", &path))?;
        let echo = blake3::hash(&bytes);
        if echo.as_bytes() != hash.as_bytes() {
            return Err(StorageError::CasCorrupt {
                hash: hash.to_string(),
                path,
            });
        }
        Ok(bytes)
    }

    pub fn get_range(&self, hash: &B3Hash, range: &Range) -> Result<Vec<u8>, StorageError> {
        let (_, path) = self.object_path(hash);
        if !self.vfs.exists(&path) {
            return Err(StorageError::CasMissing {
                hash: hash.to_string(),
            });
        }
        ranges::of_object(self.vfs.as_ref(), &path, hash, range)
    }

    pub fn size(&self, hash: &B3Hash) -> Result<u64, StorageError> {
        let (_, path) = self.object_path(hash);
        if !self.vfs.exists(&path) {
            return Err(StorageError::CasMissing {
                hash: hash.to_string(),
            });
        }
        self.vfs
            .size(&path)
            .map_err(io_err("measure cas object", &path))
    }
}

static PUTS_BEGUN: AtomicU64 = AtomicU64::new(0);

struct TmpName(String);

impl TmpName {

    fn next(hash: &B3Hash) -> TmpName {
        let count = PUTS_BEGUN.fetch_add(1, Ordering::Relaxed);
        TmpName(format!("{hash}.{}.{count}.part", std::process::id()))
    }

    fn written_here(path: &Path) -> bool {
        let pid = std::process::id().to_string();
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.split('.').nth(1))
            .is_some_and(|writer| writer == pid)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::permissions_set_readonly_false,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
    use kernel::{AxCode, Range};
    use std::fs;

    #[test]
    fn put_get_roundtrip_and_reference_hash() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let hash = cas.put(b"hello world").unwrap();
        assert_eq!(
            hash.to_string(),
            blake3::hash(b"hello world").to_hex().to_string()
        );
        assert!(cas.contains(&hash));
        assert_eq!(cas.get(&hash).unwrap(), b"hello world");
    }

    #[test]
    fn a_stored_object_states_its_size_and_a_missing_one_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let hash = cas.put(&[7_u8; 70_000]).unwrap();
        assert_eq!(cas.size(&hash).unwrap(), 70_000);
        let absent = kernel::B3Hash::digest(b"never stored");
        assert!(
            matches!(cas.size(&absent), Err(StorageError::CasMissing { .. })),
            "{:?}",
            cas.size(&absent)
        );
    }

    #[test]
    fn dedup_skips_rematerialization_and_tmp_stays_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let first = cas.put(b"same bytes").unwrap();
        let second = cas.put(b"same bytes").unwrap();
        assert_eq!(first, second);
        let tmp_entries = fs::read_dir(dir.path().join("tmp")).unwrap().count();
        assert_eq!(tmp_entries, 0, "no leftover temp files after clean puts");
    }

    #[test]
    fn opening_a_handle_sweeps_only_what_another_process_left() {
        let dir = tempfile::tempdir().unwrap();
        Cas::open(dir.path()).unwrap();
        let hash = kernel::B3Hash::from_bytes(*blake3::hash(b"in flight").as_bytes());
        let tmp = dir.path().join("tmp");
        let in_flight = tmp.join(TmpName::next(&hash).0);
        let residue = tmp.join(format!(
            "{hash}.{}.0.part",
            std::process::id().wrapping_add(1)
        ));
        fs::write(&in_flight, b"in flight").unwrap();
        fs::write(&residue, b"half").unwrap();
        Cas::open(dir.path()).unwrap();
        let left: Vec<PathBuf> = fs::read_dir(&tmp)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(left, vec![in_flight]);
    }

    fn make_writable(path: &Path) {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn get_reverifies_and_reports_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let hash = cas.put(b"precious").unwrap();
        let hex = hash.to_string();
        let path = dir.path().join("b3").join(hex.get(..2).unwrap()).join(&hex);
        make_writable(&path);
        let mut bytes = fs::read(&path).unwrap();
        bytes[0] ^= 0xFF;
        fs::write(&path, &bytes).unwrap();
        let err = cas.get(&hash).unwrap_err();
        assert_eq!(err.into_ax().code(), &AxCode::CasCorrupt);
    }

    #[test]
    fn repair_put_replaces_corrupt_bytes_and_objects_are_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let hash = cas.put(b"precious").unwrap();
        let hex = hash.to_string();
        let path = dir.path().join("b3").join(hex.get(..2).unwrap()).join(&hex);
        assert!(fs::metadata(&path).unwrap().permissions().readonly());

        make_writable(&path);
        fs::write(&path, b"corrupt").unwrap();
        assert_eq!(cas.put(b"precious").unwrap(), hash);
        assert!(
            cas.get(&hash).is_err(),
            "existence dedup keeps the hot path and cannot repair"
        );

        assert_eq!(cas.put_repair(b"precious").unwrap(), hash);
        assert_eq!(cas.get(&hash).unwrap(), b"precious");
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
    }

    #[test]
    fn missing_object_is_path_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cas = Cas::open(dir.path()).unwrap();
        let ghost = kernel::B3Hash::from_bytes([9; 32]);
        assert!(!cas.contains(&ghost));
        let err = cas.get(&ghost).unwrap_err();
        assert_eq!(err.into_ax().code(), &AxCode::PathNotFound);
    }

    #[test]
    fn byte_and_line_ranges_follow_locator_semantics() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let hash = cas.put(b"alpha\nbeta\ngamma").unwrap();

        let bytes = cas.get_range(&hash, &Range::bytes(0, 4).unwrap()).unwrap();
        assert_eq!(bytes, b"alpha");
        let tail = cas
            .get_range(&hash, &Range::bytes(11, 15).unwrap())
            .unwrap();
        assert_eq!(tail, b"gamma");

        let one = cas.get_range(&hash, &Range::lines(2, 2).unwrap()).unwrap();
        assert_eq!(one, b"beta");
        let two = cas.get_range(&hash, &Range::lines(1, 2).unwrap()).unwrap();
        assert_eq!(two, b"alpha\nbeta");
        let all = cas.get_range(&hash, &Range::lines(1, 3).unwrap()).unwrap();
        assert_eq!(all, b"alpha\nbeta\ngamma");

        for bad in [
            Range::bytes(0, 16).unwrap(),
            Range::bytes(16, 20).unwrap(),
            Range::lines(1, 4).unwrap(),
            Range::lines(4, 5).unwrap(),
        ] {
            let err = cas.get_range(&hash, &bad).unwrap_err();
            assert_eq!(err.into_ax().code(), &AxCode::InvalidArgs, "{bad:?}");
        }
    }

    #[test]
    fn a_range_read_lifts_the_range_rather_than_the_object() {
        let fs = FaultFs::new(FaultPlan {
            cut_at_op: None,
            cut_on_write: None,
            torn_tail: TornTail::None,
        });
        let mut cas = Cas::open_with(Box::new(fs.clone()), std::path::Path::new("c")).unwrap();
        let mut content = Vec::new();
        for i in 0..200_000u32 {
            content.extend_from_slice(format!("line {i} of a large object\n").as_bytes());
        }
        let object_len = u64::try_from(content.len()).unwrap();
        let hash = cas.put(&content).unwrap();

        let before = fs.bytes_read();
        let five = cas.get_range(&hash, &Range::bytes(6, 10).unwrap()).unwrap();
        assert_eq!(five, &content[6..=10]);
        let five_moved = fs.bytes_read() - before;
        assert!(
            five_moved <= 64,
            "a five-byte range moved {five_moved} bytes of a {object_len}-byte object"
        );

        let before = fs.bytes_read();
        let second = cas.get_range(&hash, &Range::lines(2, 2).unwrap()).unwrap();
        assert_eq!(second, b"line 1 of a large object");
        let moved = fs.bytes_read() - before;
        assert!(
            moved <= ranges::SCAN_CHUNK_BYTES && moved.saturating_mul(10) < object_len,
            "a line near the front moved {moved} bytes of a {object_len}-byte object"
        );

        let err = cas
            .get_range(&hash, &Range::bytes(object_len - 2, object_len).unwrap())
            .unwrap_err();
        assert_eq!(err.into_ax().code(), &AxCode::InvalidArgs);

        let before = fs.bytes_read();
        assert_eq!(cas.get(&hash).unwrap(), content);
        let whole = fs.bytes_read() - before;
        assert!(whole >= object_len);
        eprintln!(
            "cas_range_read: object {object_len} B, five-byte range {five_moved} B, \
             line 2 {moved} B, full read {whole} B"
        );
    }

    #[test]
    fn power_cut_matrix_leaves_only_tmp_halfware() {
        let content: &[u8] = b"the object under power loss";

        let fs0 = FaultFs::new(FaultPlan {
            cut_at_op: None,
            cut_on_write: None,
            torn_tail: TornTail::None,
        });
        let mut cas = Cas::open_with(Box::new(fs0.clone()), std::path::Path::new("c")).unwrap();
        cas.put(content).unwrap();
        let total_ops = fs0.op_count();

        for torn in [TornTail::None, TornTail::KeepBytes(5)] {
            for cut in 1..=total_ops {
                let fs = FaultFs::new(FaultPlan {
                    cut_at_op: Some(cut),
                    cut_on_write: None,
                    torn_tail: torn,
                });
                let dir = std::path::Path::new("c");
                let acknowledged = match Cas::open_with(Box::new(fs.clone()), dir) {
                    Ok(mut cas) => cas.put(content).is_ok(),
                    Err(_) => false,
                };

                let mut cas = match Cas::open_with(Box::new(fs.clone()), dir) {
                    Ok(cas) => cas,
                    Err(_) => Cas::open_with(Box::new(fs.clone()), dir).unwrap(),
                };
                let hash = kernel::B3Hash::from_bytes(*blake3::hash(content).as_bytes());
                if acknowledged {
                    assert_eq!(
                        cas.get(&hash).unwrap(),
                        content,
                        "cut {cut} ({torn:?}): acknowledged object must survive"
                    );
                }
                if cas.contains(&hash) {

                    assert_eq!(cas.get(&hash).unwrap(), content);
                }

                cas.put(content).unwrap();
                assert_eq!(cas.get(&hash).unwrap(), content);
            }
        }
    }
}
