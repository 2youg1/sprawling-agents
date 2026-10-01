// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The stock: one tree checked out at the trunk before any node asks for
//! it, so that placing a node's tree is a rename rather than a checkout
//! (storage-SPEC 8-35).
//!
//! A checkout waits on the real-time scanner once for every file it
//! creates, so any placement that creates the tree's files while a run
//! waits costs what the tree holds. Taking the stock over creates none:
//! the stock's directory and its registration are renamed to the node's,
//! and the two link files git keeps between them are rewritten.

use super::super::lease::FileWork;
use super::Worktrees;
use crate::error::StorageError;

impl Worktrees {
    /// Keeps one tree checked out at the trunk for the next placement to
    /// take over (storage-SPEC 8-35). Not yet built: it stocks nothing.
    ///
    /// # Errors
    /// None yet.
    pub fn stock(&self) -> Result<FileWork, StorageError> {
        Ok(FileWork::default())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use std::path::Path;

    use kernel::TimeMs;

    use super::super::super::lease::FileWork;
    use super::super::Landing;
    use super::super::Worktrees;
    use super::super::tests::{bulk_city, entries, name, owner};
    use crate::checkpoint::Checkpoint;

    fn files_under(dir: &Path) -> u64 {
        u64::try_from(std::fs::read_dir(dir).unwrap().count()).unwrap()
    }

    /// storage-SPEC 8-35: a placement from the stock creates no file,
    /// whatever the tree holds - the files were written when the stock
    /// was, where nobody waited. Judged at two sizes, so a placement
    /// whose file count grows with the tree is a failure here.
    #[test]
    fn a_placement_from_the_stock_creates_no_file_at_either_size() {
        for files in [32, 64] {
            let dir = tempfile::tempdir().unwrap();
            let trees = bulk_city(dir.path(), files);
            let city = entries(dir.path());
            let stocked = trees.stock().unwrap();
            let city_at_claim = entries(dir.path());
            let placed = trees.claim(&name("node-1"), &["bulk".to_owned()]).unwrap();
            assert_eq!(
                (
                    stocked,
                    placed.work(),
                    files_under(&placed.path().join("bulk"))
                ),
                (
                    FileWork {
                        created: files,
                        walked: city,
                        ..FileWork::default()
                    },
                    FileWork {
                        walked: city_at_claim + entries(placed.path()),
                        ..FileWork::default()
                    },
                    files,
                ),
                "a stock and the placement that takes it at {files} files"
            );
        }
    }

    /// A stock made before the trunk moved is placed at the trunk: the
    /// placement writes what the trunk changed since, the node's tree
    /// is on the node's own branch, and what the node offers from it
    /// merges like the offer of any other tree.
    #[test]
    fn a_stock_behind_the_trunk_is_placed_at_the_trunk_and_merges() {
        let dir = tempfile::tempdir().unwrap();
        let trees = bulk_city(dir.path(), 8);
        trees.stock().unwrap();
        std::fs::write(dir.path().join("bulk").join("file-0000.txt"), b"moved on").unwrap();
        Checkpoint::open(dir.path())
            .unwrap()
            .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: bulk")
            .unwrap();

        let bulk = ["bulk".to_owned()];
        let placed = trees.claim(&name("node-1"), &bulk).unwrap();
        let tree = placed.path().to_path_buf();
        let repo = git2::Repository::open(&tree).unwrap();
        assert_eq!(
            (
                placed.work().created,
                placed.work().rewritten,
                std::fs::read_to_string(tree.join("bulk").join("file-0000.txt")).unwrap(),
                repo.head().unwrap().name().ok().map(str::to_owned),
                trees.live().unwrap(),
            ),
            (
                0,
                1,
                "moved on".to_owned(),
                Some("refs/heads/node-1".to_owned()),
                vec![name("node-1")],
            ),
            "the stock is the node's tree, at the trunk, on the node's branch"
        );

        std::fs::write(tree.join("bulk").join("file-0001.txt"), b"from the node").unwrap();
        Checkpoint::open(&tree)
            .unwrap()
            .land(&bulk, TimeMs::new(3_000), &owner(), "offer: bulk")
            .unwrap();
        trees.release(placed).unwrap();
        let of = owner();
        trees
            .plan_merge(&name("node-1"))
            .unwrap()
            .apply(&Landing {
                t: TimeMs::new(4_000),
                of: &of,
                subject: "merge: node-1",
                reviewed_by_person: false,
            })
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("bulk").join("file-0001.txt")).unwrap(),
            "from the node"
        );
    }

    /// The stock is nobody's tree: no node lists it, the sweep at open
    /// leaves it, and the first placement after that takes it over.
    #[test]
    fn the_stock_is_no_nodes_tree_and_outlives_the_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let trees = bulk_city(dir.path(), 8);
        trees.stock().unwrap();
        let swept = Worktrees::sweep_abandoned(dir.path(), &[]).unwrap();
        let listed = trees.live().unwrap();
        let placed = trees.claim(&name("node-1"), &[]).unwrap();
        assert_eq!(
            (swept, listed, placed.work().created),
            (Vec::new(), Vec::new(), 0),
            "the stock survives the sweep and the next placement creates nothing"
        );
    }
}
