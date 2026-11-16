// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::super::lease::{FileWork, WorktreeLease};
use super::super::name::WorktreeName;
use super::super::weight::Weight;
use super::Worktrees;
use super::kept::restore;
use super::open_tree;
use crate::error::StorageError;

pub(super) const STOCK: &str = "+spare";

const STOCKING: &str = "being stocked by sprawling";

enum Stock {
    Absent,

    Busy,

    Broken,
    Ready(git2::Worktree, Links),
}

struct Links {
    tree: PathBuf,
    admin: PathBuf,

    dot_git: String,

    gitdir: String,
}

impl Links {
    fn of(&self, name: &str) -> (String, String) {
        (
            format!("{}worktrees/{name}/\n", self.dot_git),
            format!("{}/{name}/.git\n", self.gitdir),
        )
    }
}

impl Worktrees {
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

    fn stock_standing(&self) -> Result<Stock, StorageError> {
        let admin = self.repo.commondir().join(super::WORKTREE_DIR).join(STOCK);
        let tree = match self.repo.find_worktree(STOCK) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Stock::Absent),

            Err(_) if admin.is_dir() => return Ok(Stock::Broken),
            Err(err) => return Err(git("find the stock", STOCK, &err)),
        };
        let lock = match tree.is_locked() {
            Ok(lock) => lock,

            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Stock::Absent),
            Err(err) => return Err(git("read the stock's lock", STOCK, &err)),
        };
        if let git2::WorktreeLockStatus::Locked(_) = lock {
            return Ok(Stock::Busy);
        }
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

        let work = restore(tree, STOCK, &[]).and_then(|work| cache_trees(tree).map(|()| work));
        tree.unlock()
            .map_err(|err| git("unlock the stock", STOCK, &err))?;
        work
    }

    fn take_back(&self, refusal: StorageError) -> StorageError {
        match self.forget(STOCK) {
            Ok(()) => refusal,
            Err(left) => StorageError::Worktree {
                op: "take back a stock that did not check out",
                detail: format!("{left}; the checkout failed with: {refusal}"),
            },
        }
    }

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

fn cache_trees(tree: &git2::Worktree) -> Result<(), StorageError> {
    let repo = open_tree(tree).map_err(|err| git("open the stock", STOCK, &err))?;
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
    use super::STOCK;
    use crate::checkpoint::Checkpoint;

    fn files_under(dir: &Path) -> u64 {
        u64::try_from(std::fs::read_dir(dir).unwrap().count()).unwrap()
    }

    #[test]
    fn a_registration_git_will_not_read_is_stocked_again() {
        let dir = tempfile::tempdir().unwrap();
        let trees = bulk_city(dir.path(), 8);
        trees.stock().unwrap();
        let registration = trees
            .repo
            .commondir()
            .join(super::super::WORKTREE_DIR)
            .join(STOCK);
        std::fs::remove_file(registration.join("gitdir")).unwrap();

        trees.stock().unwrap();
        let placed = trees.claim(&name("node-1"), &["bulk".to_owned()]).unwrap();
        assert_eq!(
            std::fs::read_dir(placed.path().join("bulk"))
                .unwrap()
                .count(),
            8,
            "the stock is whole again after the cut registration was taken back"
        );
    }

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

    #[test]
    fn a_room_placed_while_the_stock_is_being_made_checks_out_once_and_the_stock_survives() {
        for files in [8, 16, 32] {
            let dir = tempfile::tempdir().unwrap();
            let trees = bulk_city(dir.path(), files);
            trees.stock().unwrap();
            let first = trees.claim(&name("node-1"), &[]).unwrap().work().created;
            let restocked = trees.stock().unwrap().created;
            let making = git2::Repository::open(dir.path())
                .unwrap()
                .find_worktree(super::STOCK)
                .unwrap();
            making.lock(Some(super::STOCKING)).unwrap();
            let during = trees.claim(&name("node-2"), &[]).unwrap().work().created;
            making.unlock().unwrap();
            let refreshed = trees.stock().unwrap().created;
            let after = trees.claim(&name("node-3"), &[]).unwrap().work().created;
            assert_eq!(
                (first, restocked, during, refreshed, after),
                (0, files, files, 0, 0),
                "files created per step at {files} files"
            );
        }
    }

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
