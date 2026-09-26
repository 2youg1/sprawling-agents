// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A tree kept for its node between runs: whether somebody holds it,
//! and how it is taken back into use.

use super::super::lease::WorktreeLease;
use super::super::name::WorktreeName;
use super::super::weight::measure;
use super::Worktrees;
use crate::error::MemoryError;

/// Why a tree is locked: a run holds it.
const LEASE_REASON: &str = "held by a sprawling run";

/// What [`Worktrees::claim`] finds under a name.
pub(super) enum Standing {
    Held,
    Kept(git2::Worktree),
    Absent,
}

impl Worktrees {
    /// Lifts every lease a previous writer of this city left behind.
    ///
    /// A city has one writer, and `_writer` is the proof that the caller
    /// is it: while it is held, no lock can belong to a run that is
    /// still alive, so every lock is one a dead process never gave
    /// back. A city with no repository has no trees to free.
    ///
    /// # Errors
    /// Propagates a repository that cannot list, read or unlock its
    /// trees.
    pub fn lift_abandoned_leases(
        city_root: &std::path::Path,
        _writer: &crate::JsonlLedger,
    ) -> Result<(), MemoryError> {
        let refuse = |op: &'static str, err: git2::Error| MemoryError::Worktree {
            op,
            detail: format!("{}: {err}", city_root.display()),
        };
        let repo = match git2::Repository::open(city_root) {
            Ok(repo) => repo,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),
            Err(err) => return Err(refuse("open the city repository", err)),
        };
        let names = repo
            .worktrees()
            .map_err(|err| refuse("list worktrees", err))?;
        for name in names.iter().flatten().flatten() {
            let tree = repo
                .find_worktree(name)
                .map_err(|err| refuse("find a worktree", err))?;
            let lock = tree
                .is_locked()
                .map_err(|err| refuse("read a worktree lock", err))?;
            match lock {
                git2::WorktreeLockStatus::Locked(_) => tree
                    .unlock()
                    .map_err(|err| refuse("unlock a worktree", err))?,
                git2::WorktreeLockStatus::Unlocked => {}
            }
        }
        Ok(())
    }

    /// Where `name` stands: held under a lock, kept on disk for its
    /// node, or not there at all.
    ///
    /// A registration with no directory under it is what an interrupted
    /// placement or a person deleting the tree by hand leaves, and it is
    /// taken back here - the same reflex the index has about a cache it
    /// doubts - so that `E_WORKTREE_BUSY` keeps meaning that somebody is
    /// working.
    pub(super) fn standing(&self, name: &WorktreeName) -> Result<Standing, MemoryError> {
        let tree = match self.repo.find_worktree(name.as_str()) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Standing::Absent),
            Err(err) => {
                return Err(MemoryError::Worktree {
                    op: "find a worktree",
                    detail: format!("{}: {err}", name.as_str()),
                });
            }
        };
        if !tree.path().exists() {
            self.forget(name)?;
            return Ok(Standing::Absent);
        }
        let lock = tree.is_locked().map_err(|err| MemoryError::Worktree {
            op: "read a worktree lock",
            detail: format!("{}: {err}", name.as_str()),
        })?;
        Ok(match lock {
            git2::WorktreeLockStatus::Locked(_) => Standing::Held,
            git2::WorktreeLockStatus::Unlocked => Standing::Kept(tree),
        })
    }

    /// Takes a kept tree back into use: the node's branch moved up to
    /// the trunk when the trunk already holds all of it, the index read
    /// back from that branch head, every tracked file under `scopes`
    /// forced to it and every untracked one there removed, then the
    /// lock. Git rewrites only the files that differ, and only inside
    /// the scope, so a node's second run costs what it left behind in
    /// its own scope, not the size of the city.
    ///
    /// The index is read back whole before the narrowed checkout because
    /// that checkout moves only the entries it writes: an entry outside
    /// the scope left at the last run's commit would be committed again
    /// by the next scoped offer, taking back what the trunk changed there.
    pub(super) fn reattach(
        &self,
        name: &WorktreeName,
        tree: &git2::Worktree,
        scopes: &[String],
    ) -> Result<WorktreeLease, MemoryError> {
        let refuse = |op: &'static str, err: git2::Error| MemoryError::Worktree {
            op,
            detail: format!("{}: {err}", name.as_str()),
        };
        self.follow_trunk(name)?;
        let repo = git2::Repository::open_from_worktree(tree)
            .map_err(|err| refuse("open a kept worktree", err))?;
        let head = repo
            .head()
            .and_then(|head| head.peel_to_tree())
            .map_err(|err| refuse("read a kept worktree's branch", err))?;
        let mut index = repo
            .index()
            .map_err(|err| refuse("read a kept worktree's index", err))?;
        index
            .read_tree(&head)
            .and_then(|()| index.write())
            .map_err(|err| refuse("reset a kept worktree's index", err))?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force().remove_untracked(true);
        for spec in crate::checkpoint::Checkpoint::pathspecs(scopes) {
            checkout.path(spec);
        }
        repo.checkout_head(Some(&mut checkout))
            .map_err(|err| refuse("reset a kept worktree", err))?;
        tree.lock(Some(LEASE_REASON))
            .map_err(|err| refuse("lock a worktree", err))?;
        Ok(WorktreeLease {
            name: name.clone(),
            path: tree.path().to_path_buf(),
            disk: measure(tree.path())?,
        })
    }

    /// Moves `name`'s branch to the trunk when the trunk descends from
    /// it. A branch with work the trunk lacks stays where it is: that
    /// work is still waiting for its merge.
    fn follow_trunk(&self, name: &WorktreeName) -> Result<(), MemoryError> {
        let refuse = |op: &'static str, err: git2::Error| MemoryError::Worktree {
            op,
            detail: format!("{}: {err}", name.as_str()),
        };
        let mut branch = self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
            .map_err(|err| refuse("find a node branch", err))?;
        let tip = branch
            .get()
            .peel_to_commit()
            .map_err(|err| refuse("read a node branch", err))?
            .id();
        let trunk = self
            .repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|err| refuse("read the city trunk", err))?
            .id();
        let behind = tip != trunk
            && self
                .repo
                .graph_descendant_of(trunk, tip)
                .map_err(|err| refuse("judge a fast-forward", err))?;
        if behind {
            branch
                .get_mut()
                .set_target(trunk, "follow the trunk")
                .map_err(|err| refuse("move a node branch", err))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::super::Landing;
    use super::super::tests::{city, name, owner};
    use crate::checkpoint::Checkpoint;
    use kernel::TimeMs;
    use std::path::Path;

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    /// A kept tree claimed again writes only its scope, and what the node
    /// offers from it moves only that scope: the trunk's work elsewhere,
    /// which this tree never checked out, lands in the city untouched.
    #[test]
    fn a_kept_tree_claimed_again_checks_out_only_its_scope_and_offers_only_that() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        let city_docs = dir.path().join("docs").join("plan.md");
        let land_city = |t: u64| {
            Checkpoint::open(dir.path())
                .unwrap()
                .land(&[], TimeMs::new(t), &owner(), "checkpoint: city")
                .unwrap()
        };
        std::fs::create_dir_all(city_docs.parent().unwrap()).unwrap();
        std::fs::write(
            &city_docs, b"v1
",
        )
        .unwrap();
        land_city(1_500);
        let lab = ["lab".to_owned()];
        trees
            .release(trees.claim(&name("node-1"), &lab).unwrap())
            .unwrap();
        std::fs::write(
            &city_docs, b"v2
",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("lab").join("notes.md"),
            b"second
",
        )
        .unwrap();
        let trunk = land_city(2_000);

        let again = trees.claim(&name("node-1"), &lab).unwrap();
        assert_eq!(
            (
                read(&again.path().join("lab").join("notes.md")),
                read(&again.path().join("docs").join("plan.md"))
            ),
            (
                "second
"
                .to_owned(),
                "v1
"
                .to_owned()
            ),
            "the scope follows the trunk and nothing outside it is written"
        );

        std::fs::write(
            again.path().join("lab").join("notes.md"),
            b"from the node
",
        )
        .unwrap();
        Checkpoint::open(again.path())
            .unwrap()
            .land(&lab, TimeMs::new(3_000), &owner(), "offer: lab")
            .unwrap();
        let of = owner();
        trees
            .plan_merge(again.name())
            .unwrap()
            .apply(&Landing {
                t: TimeMs::new(4_000),
                of: &of,
                subject: "merge: node-1",
                reviewed_by_person: false,
            })
            .unwrap();
        let repo = git2::Repository::open(dir.path()).unwrap();
        let before = repo
            .find_commit(git2::Oid::from_str(&trunk).unwrap())
            .unwrap()
            .tree()
            .unwrap();
        let after = repo.head().unwrap().peel_to_tree().unwrap();
        let moved: Vec<String> = repo
            .diff_tree_to_tree(Some(&before), Some(&after), None)
            .unwrap()
            .deltas()
            .filter_map(|delta| delta.new_file().path().map(|p| p.display().to_string()))
            .collect();
        assert_eq!(
            (moved, read(&city_docs)),
            (
                vec!["lab/notes.md".to_owned()],
                "v2
"
                .to_owned()
            ),
            "the merge moves the scope alone"
        );
    }

    /// A node's next run finds its tree where it left it: releasing gives
    /// the lease back and keeps the files, so the next claim neither
    /// measures the city nor checks the whole tree out again.
    #[test]
    fn a_released_tree_stays_on_disk_for_the_nodes_next_run() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        let lease = trees.claim(&name("node-1"), &[]).unwrap();
        let path = lease.path().to_path_buf();
        let notes = path.join("lab").join("notes.md");
        let written = std::fs::metadata(&notes).unwrap().modified().unwrap();

        trees.release(lease).unwrap();
        assert!(notes.exists(), "a released tree keeps its files");
        assert_eq!(trees.live().unwrap(), vec![name("node-1")]);

        let again = trees.claim(&name("node-1"), &[]).unwrap();
        assert_eq!(again.path(), path.as_path());
        assert_eq!(
            std::fs::metadata(&notes).unwrap().modified().unwrap(),
            written,
            "a file the node's branch already holds is not written again"
        );
        assert!(dir.path().join("lab").join("notes.md").exists());
    }

    /// A kept tree whose node has nothing unmerged follows the trunk: a
    /// node that starts from where the city was when it last ran would do
    /// its next work on files somebody has since changed, and every merge
    /// of it would be refused as stale.
    #[test]
    fn a_kept_tree_with_nothing_unmerged_starts_from_the_trunk_as_it_now_stands() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        trees
            .release(trees.claim(&name("node-1"), &[]).unwrap())
            .unwrap();
        std::fs::write(
            dir.path().join("lab").join("notes.md"),
            b"second
    ",
        )
        .unwrap();
        Checkpoint::open(dir.path())
            .unwrap()
            .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: lab")
            .unwrap();

        let again = trees.claim(&name("node-1"), &[]).unwrap();
        assert_eq!(
            std::fs::read_to_string(again.path().join("lab").join("notes.md")).unwrap(),
            "second
    "
        );
    }
}
