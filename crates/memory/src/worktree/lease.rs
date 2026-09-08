// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Worktree leases: a held tree and its measure.

use std::path::{Path, PathBuf};

use kernel::{ByteLen, Payload};
use serde_json::{Map, Value};

use crate::error::MemoryError;

use super::name::WorktreeName;

/// A live tree: what it is called, where it is, and what it costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeLease {
    pub(crate) name: WorktreeName,
    pub(crate) path: PathBuf,
    pub(crate) disk: ByteLen,
}

impl WorktreeLease {
    #[must_use]
    pub fn name(&self) -> &WorktreeName {
        &self.name
    }

    /// The root a run in this node writes against.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What the tree occupied when it was opened. `status` reports this,
    /// which is why it is measured rather than estimated.
    #[must_use]
    pub fn disk(&self) -> ByteLen {
        self.disk
    }

    /// The `worktree_opened` payload: a name and a size, no path. An
    /// absolute path is a fact about this machine, and a history that
    /// carries one does not survive being moved to another.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn opened_payload(&self) -> Result<Payload, MemoryError> {
        let mut map = Map::new();
        map.insert(
            "name".to_owned(),
            Value::String(self.name.as_str().to_owned()),
        );
        map.insert(
            "disk_bytes".to_owned(),
            Value::Number(self.disk.get().into()),
        );
        Payload::new(map).map_err(|source| MemoryError::Draft { source })
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
                "lab",
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
        let lease = trees.claim(&name("node-1")).unwrap();
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
