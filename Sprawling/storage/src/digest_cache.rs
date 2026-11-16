// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{B3Hash, Payload};
use serde_json::{Map, Value};

use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

pub struct DigestCache {
    dir: PathBuf,
    fs: RealFs,
}

impl DigestCache {
    pub fn open(dir: &Path) -> Result<DigestCache, StorageError> {
        let mut fs = RealFs::new();
        fs.create_dir_all(dir)
            .map_err(io_err("create digest cache dir", dir))?;
        let tmp = dir.join("tmp");
        fs.create_dir_all(&tmp)
            .map_err(io_err("create digest cache tmp", &tmp))?;

        let leftovers = fs.list(&tmp).map_err(io_err("list digest tmp", &tmp))?;
        for leftover in leftovers {
            fs.remove_file(&leftover)
                .map_err(io_err("sweep digest tmp", &leftover))?;
        }
        Ok(DigestCache {
            dir: dir.to_path_buf(),
            fs,
        })
    }

    fn entry_path(&self, content: &B3Hash) -> PathBuf {
        self.dir.join(format!("{content}.json"))
    }

    pub fn put(&mut self, content: &B3Hash, tree_json: &[u8]) -> Result<(), StorageError> {
        let final_path = self.entry_path(content);
        if self.fs.exists(&final_path) {
            return Ok(());
        }
        let tmp_path = self.dir.join("tmp").join(format!("{content}.part"));
        if self.fs.exists(&tmp_path) {
            self.fs
                .truncate(&tmp_path, 0)
                .map_err(io_err("reset digest tmp", &tmp_path))?;
        }
        self.fs
            .append(&tmp_path, tree_json)
            .map_err(io_err("write digest entry", &tmp_path))?;
        self.fs
            .sync_data(&tmp_path)
            .map_err(io_err("sync digest entry", &tmp_path))?;
        self.fs
            .rename(&tmp_path, &final_path)
            .map_err(io_err("publish digest entry", &final_path))?;
        self.fs
            .sync_dir(&self.dir)
            .map_err(io_err("sync digest cache dir", &self.dir))?;
        Ok(())
    }

    pub fn get(&self, content: &B3Hash) -> Result<Option<Vec<u8>>, StorageError> {
        match self.fs.read(&self.entry_path(content)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(io_err("read digest entry", &self.entry_path(content))(err)),
        }
    }

    pub fn invalidate(&mut self, content: &B3Hash, reason: &str) -> Result<Payload, StorageError> {
        let path = self.entry_path(content);
        let existed = self.fs.exists(&path);
        if existed {
            self.fs
                .remove_file(&path)
                .map_err(io_err("remove digest entry", &path))?;
        }
        let mut map = Map::new();
        map.insert("content".to_owned(), Value::String(content.to_string()));
        map.insert("reason".to_owned(), Value::String(reason.to_owned()));
        map.insert("existed".to_owned(), Value::Bool(existed));
        Payload::new(map).map_err(|source| StorageError::Draft { source })
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

    #[test]
    fn the_same_content_digests_once_for_life() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cache = DigestCache::open(tmp.path()).unwrap();
        let content = B3Hash::digest(b"a long document");
        cache.put(&content, br#"{"tree":"first"}"#).unwrap();

        cache.put(&content, br#"{"tree":"second"}"#).unwrap();
        assert_eq!(
            cache.get(&content).unwrap().as_deref(),
            Some(&br#"{"tree":"first"}"#[..])
        );
    }

    #[test]
    fn a_miss_is_absence_not_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = DigestCache::open(tmp.path()).unwrap();
        let never = B3Hash::digest(b"never stored");
        assert_eq!(cache.get(&never).unwrap(), None);
    }

    #[test]
    fn invalidation_removes_the_entry_and_says_whether_it_was_there() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cache = DigestCache::open(tmp.path()).unwrap();
        let content = B3Hash::digest(b"doc");
        cache.put(&content, b"{}").unwrap();
        let payload = cache.invalidate(&content, "source edited").unwrap();
        let value = serde_json::to_value(&payload).unwrap();
        assert_eq!(value["existed"], true);
        assert_eq!(value["reason"], "source edited");
        assert_eq!(cache.get(&content).unwrap(), None);

        let payload = cache.invalidate(&content, "again").unwrap();
        assert_eq!(serde_json::to_value(&payload).unwrap()["existed"], false);
    }

    #[test]
    fn a_crash_between_write_and_publish_leaves_nothing_at_the_final_name() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cache = DigestCache::open(tmp.path()).unwrap();
        let content = B3Hash::digest(b"doc");

        std::fs::write(
            tmp.path().join("tmp").join(format!("{content}.part")),
            b"half",
        )
        .unwrap();
        assert_eq!(
            cache.get(&content).unwrap(),
            None,
            "no half entry is visible"
        );

        cache.put(&content, b"{\"tree\":\"ok\"}").unwrap();
        assert_eq!(
            cache.get(&content).unwrap().as_deref(),
            Some(&b"{\"tree\":\"ok\"}"[..])
        );
    }
}
