// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The stock: one tree checked out at the trunk before any node asks for
//! it, so that placing a node's tree is a rename rather than a checkout
//! (`crates/storage/spec/Worktree/Trees/Stock.lean` §8-35).
//!
//! A checkout waits on the real-time scanner once for every file it
//! creates, so any placement that creates the tree's files while a run
//! waits costs what the tree holds. Taking the stock over creates none:
//! the stock's directory and its registration are renamed to the node's,
//! and the two link files git keeps between them are rewritten.

use std::path::{Path, PathBuf};

use super::super::lease::{FileWork, WorktreeLease};
use super::super::name::WorktreeName;
use super::super::weight::Weight;
use super::Worktrees;
use super::kept::restore;
use crate::error::StorageError;

/// The stock's id: its registration, its directory under the tree home
/// and its branch. `+` is outside `WorktreeName`'s alphabet, so no node
/// can be named this and no claim, release or merge reaches the stock;
/// git admits it in a branch name.
pub(super) const STOCK: &str = "+spare";

/// Why a stock is locked: it is being checked out, and a tree half
/// written is not one to hand to a node.
const STOCKING: &str = "being stocked by sprawling";

/// What the tree home holds under the stock's id.
enum Stock {
    Absent,
    /// Being checked out by another call.
    Busy,
    /// Registered, but its directory or a link file is not what this
    /// module writes: taken back and placed anew rather than handed out.
    Broken,
    Ready(git2::Worktree, Links),
}

/// The two link files between the stock's directory and its
/// registration, as git wrote them, and where each lives.
struct Links {
    tree: PathBuf,
    admin: PathBuf,
    /// The tree's `.git` file, up to the stock's id.
    dot_git: String,
    /// The registration's `gitdir` file, up to the stock's id.
    gitdir: String,
}

impl Links {
    /// The two files as they read once the stock is `name`'s.
    fn of(&self, name: &str) -> (String, String) {
        (
            format!("{}worktrees/{name}/\n", self.dot_git),
            format!("{}/{name}/.git\n", self.gitdir),
        )
    }
}

impl Worktrees {
    /// Keeps one tree checked out at the trunk for the next placement to
    /// take over, and answers what that cost the filesystem: a whole
    /// checkout when there is no stock, and the files the trunk changed
    /// since the stock was last brought to it otherwise. A stock that
    /// another call is checking out is left to it, at no cost.
    ///
    /// Call it where nobody waits on it: its cost is the placement's.
    ///
    /// # Errors
    /// Refuses a city whose working tree exceeds the ceiling and a
    /// repository with no commit, and propagates whatever git or the
    /// disk refuses while the stock is checked out.
    pub fn stock(&self) -> Result<FileWork, StorageError> {
        match self.stock_standing()? {
            Stock::Absent => self.place_stock(),
            Stock::Busy => Ok(FileWork::default()),
            Stock::Broken => {
                self.forget(STOCK)?;
                self.place_stock()
            }
            Stock::Ready(tree, _) => self.refresh_stock(&tree),
        }
    }

    /// Takes the stock over as `name`'s tree and lends it, or answers
    /// `None` when there is no stock to take: none was made, another
    /// call is making it, or another placement took it first. `city` is
    /// the walk the ceiling was judged on.
    ///
    /// The directory is renamed first, because that rename is the one
    /// step two placements cannot both make; every later step leaves a
    /// state the next claim of `name` takes back (`crates/storage/spec/Worktree/Trees/Stock.lean` §8-35).
    pub(super) fn adopt(
        &self,
        name: &WorktreeName,
        city: &Weight,
    ) -> Result<Option<WorktreeLease>, StorageError> {
        let Stock::Ready(_, links) = self.stock_standing()? else {
            return Ok(None);
        };
        let target = self.home.join(name.as_str());
        crate::alias::WriteTarget::within("place a worktree", &self.city_root, &target)?;
        self.branch_at_trunk(name)?;
        match std::fs::rename(&links.tree, &target) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(io("take the stock", &links.tree, source)),
        }
        let (dot_git, gitdir) = links.of(name.as_str());
        let admin = links.admin.with_file_name(name.as_str());
        write(&target.join(".git"), &dot_git)?;
        std::fs::rename(&links.admin, &admin)
            .map_err(|source| io("register the stock as a node's tree", &links.admin, source))?;
        write(&admin.join("gitdir"), &gitdir)?;
        let tree = self
            .repo
            .find_worktree(name.as_str())
            .map_err(|err| git("find a taken stock", name.as_str(), &err))?;
        let lease = self.reattach(name, &tree, &[])?;
        Ok(Some(WorktreeLease {
            work: FileWork {
                walked: lease.work.walked.saturating_add(city.walked),
                ..lease.work
            },
            ..lease
        }))
    }

    /// Where the stock stands, read from git and the disk.
    fn stock_standing(&self) -> Result<Stock, StorageError> {
        let tree = match self.repo.find_worktree(STOCK) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Stock::Absent),
            Err(err) => return Err(git("find the stock", STOCK, &err)),
        };
        let lock = tree
            .is_locked()
            .map_err(|err| git("read the stock's lock", STOCK, &err))?;
        if let git2::WorktreeLockStatus::Locked(_) = lock {
            return Ok(Stock::Busy);
        }
        let admin = self.repo.commondir().join("worktrees").join(STOCK);
        let read = |path: &Path| match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io("read the stock's links", path, source)),
        };
        let dot_git = read(&tree.path().join(".git"))?;
        let gitdir = read(&admin.join("gitdir"))?;
        let links = dot_git
            .as_deref()
            .and_then(|text| text.strip_suffix(&format!("worktrees/{STOCK}/\n")))
            .zip(
                gitdir
                    .as_deref()
                    .and_then(|text| text.strip_suffix(&format!("/{STOCK}/.git\n"))),
            );
        Ok(match links {
            Some((dot_git, gitdir)) => {
                let links = Links {
                    tree: tree.path().to_path_buf(),
                    admin,
                    dot_git: dot_git.to_owned(),
                    gitdir: gitdir.to_owned(),
                };
                Stock::Ready(tree, links)
            }
            None => Stock::Broken,
        })
    }

    /// Checks a new stock out at the trunk, locked until it is whole.
    fn place_stock(&self) -> Result<FileWork, StorageError> {
        let city = self.refuse_oversized(STOCK)?;
        self.clear_unregistered(STOCK)?;
        let trunk = self
            .repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|err| git("read the city trunk", STOCK, &err))?;
        let branch = self
            .repo
            .branch(STOCK, &trunk, true)
            .map_err(|err| git("branch the stock", STOCK, &err))?;
        let (_, written) = match self.check_out_tree(STOCK, Some(branch.get())) {
            Ok(placed) => placed,
            // A registration left behind locked would read as a stocking
            // in progress for the rest of this serving.
            Err(refusal) => return Err(self.take_back(refusal)),
        };
        let tree = self
            .repo
            .find_worktree(STOCK)
            .map_err(|err| git("find the stock", STOCK, &err))?;
        cache_trees(&tree)?;
        tree.unlock()
            .map_err(|err| git("unlock the stock", STOCK, &err))?;
        Ok(FileWork {
            created: written.files,
            walked: city.walked,
            ..FileWork::default()
        })
    }

    /// Brings a whole stock up to the trunk, locked while it moves.
    fn refresh_stock(&self, tree: &git2::Worktree) -> Result<FileWork, StorageError> {
        tree.lock(Some(STOCKING))
            .map_err(|err| git("lock the stock", STOCK, &err))?;
        let trunk = self
            .repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|err| git("read the city trunk", STOCK, &err))?;
        self.repo
            .branch(STOCK, &trunk, true)
            .map_err(|err| git("move the stock to the trunk", STOCK, &err))?;
        // Unlocked even when the restore failed: a stock half moved is
        // whole again after the restore a placement makes of it, while a
        // lock left behind would read as a stocking in progress.
        let work = restore(tree, STOCK, &[]).and_then(|work| cache_trees(tree).map(|()| work));
        tree.unlock()
            .map_err(|err| git("unlock the stock", STOCK, &err))?;
        work
    }

    /// Unregisters a stock whose checkout failed and answers the
    /// failure, or the failure to unregister it with the checkout's
    /// failure in its detail.
    fn take_back(&self, refusal: StorageError) -> StorageError {
        match self.forget(STOCK) {
            Ok(()) => refusal,
            Err(left) => StorageError::Worktree {
                op: "take back a stock that did not check out",
                detail: format!("{left}; the checkout failed with: {refusal}"),
            },
        }
    }

    /// Makes `name`'s branch at the trunk unless the node already has
    /// one: a node's branch is its line of work, and the stock is only
    /// where its files come from.
    fn branch_at_trunk(&self, name: &WorktreeName) -> Result<(), StorageError> {
        match self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
        {
            Ok(_) => Ok(()),
            Err(err) if err.code() == git2::ErrorCode::NotFound => {
                let trunk = self
                    .repo
                    .head()
                    .and_then(|head| head.peel_to_commit())
                    .map_err(|err| git("read the city trunk", name.as_str(), &err))?;
                self.repo
                    .branch(name.as_str(), &trunk, false)
                    .map(drop)
                    .map_err(|err| git("branch a worktree", name.as_str(), &err))
            }
            Err(err) => Err(git("find a node branch", name.as_str(), &err)),
        }
    }
}

/// Reads the stock's index back from its head and writes it, so that the
/// index carries the tree cache: the placement that takes the stock over
/// then learns that the index writes the head's tree without hashing a
/// single tree (`kept::index_writes_tree`). Reading back keeps the stat
/// data of every entry whose blob and mode are unchanged, and writing the
/// index after the checkout's last file leaves no entry racily clean.
fn cache_trees(tree: &git2::Worktree) -> Result<(), StorageError> {
    let repo = git2::Repository::open_from_worktree(tree)
        .map_err(|err| git("open the stock", STOCK, &err))?;
    let head = repo
        .head()
        .and_then(|head| head.peel_to_tree())
        .map_err(|err| git("read the stock's branch", STOCK, &err))?;
    let mut index = repo
        .index()
        .map_err(|err| git("read the stock's index", STOCK, &err))?;
    index
        .read_tree(&head)
        .and_then(|()| index.write())
        .map_err(|err| git("cache the stock's trees", STOCK, &err))
}

/// Writes one link file whole.
fn write(path: &Path, text: &str) -> Result<(), StorageError> {
    std::fs::write(path, text).map_err(|source| io("rewrite a worktree link", path, source))
}

fn io(op: &'static str, path: &Path, source: std::io::Error) -> StorageError {
    StorageError::Io {
        op,
        path: path.to_path_buf(),
        source,
    }
}

fn git(op: &'static str, id: &str, err: &git2::Error) -> StorageError {
    StorageError::Worktree {
        op,
        detail: format!("{id}: {err}"),
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

    /// `crates/storage/spec/Worktree/Trees/Stock.lean` §8-35: a placement from the stock creates no file,
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
