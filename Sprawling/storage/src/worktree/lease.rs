// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::event::record::WorktreeOpened;
use kernel::{ByteLen, Payload};

use crate::error::StorageError;

use super::name::WorktreeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeLease {
    pub(crate) name: WorktreeName,
    pub(crate) path: PathBuf,
    pub(crate) disk: ByteLen,
    pub(crate) work: FileWork,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileWork {

    pub created: u64,

    pub rewritten: u64,

    pub removed: u64,

    pub walked: u64,
}

impl WorktreeLease {
    #[must_use]
    pub fn name(&self) -> &WorktreeName {
        &self.name
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn disk(&self) -> ByteLen {
        self.disk
    }

    #[must_use]
    pub fn work(&self) -> FileWork {
        self.work
    }

    pub fn opened_payload(&self) -> Result<Payload, StorageError> {
        Payload::of(&WorktreeOpened {
            name: self.name.as_str().to_owned(),
            disk_bytes: self.disk,
        })
        .map_err(|source| StorageError::Draft { source })
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
    use super::super::trees::Worktrees;
    use serde_json::Value;

    use super::*;
    use crate::checkpoint::Checkpoint;
    use kernel::TimeMs;
    use kernel::consts_policy::WORKTREE_MAX_BYTES;
    fn city(dir: &Path) -> Worktrees {
        std::fs::create_dir_all(dir.join("lab")).unwrap();
        std::fs::write(dir.join("lab").join("notes.md"), b"first\n").unwrap();
        let mut checkpoint = Checkpoint::open(dir).unwrap();
        checkpoint
            .ensure_base(
                &["lab".to_owned()],
                TimeMs::new(1_000),
                &super::super::trees::tests::owner(),
            )
            .unwrap();
        Worktrees::open(dir).unwrap()
    }
    fn name(raw: &str) -> WorktreeName {
        WorktreeName::parse(raw).unwrap()
    }

    #[test]
    fn the_tree_is_measured_rather_than_estimated() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        let lease = trees.claim(&name("node-1"), &[]).unwrap();
        assert!(
            lease.disk().get() >= 6,
            "the checked-out file has six bytes"
        );
        assert!(lease.disk().get() < WORKTREE_MAX_BYTES);

        let payload = lease.opened_payload().unwrap();
        let map = payload.as_map();
        assert_eq!(map.get("name").and_then(Value::as_str), Some("node-1"));
        assert!(map.contains_key("disk_bytes"));
        assert!(
            !map.contains_key("path"),
            "an absolute path is a fact about this machine, not about the city"
        );
    }
}
