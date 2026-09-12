// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What has not been fenced yet: the branch, its drift from the
//! upstream it tracks, and the files that differ from the last commit.
//!
//! **Why this is not [`crate::between`] with a different far end.**
//! That function compares two points a caller already holds and answers
//! nothing about where the repository itself stands. Which branch is
//! checked out, whether it has an upstream, and how far it has run
//! ahead of it are facts about the repository rather than about a pair
//! of trees — and they are the three a person asks before they ask
//! which files moved. The rows are read through `changes::collect` all
//! the same, so what a changed file is keeps one answer.
//!
//! **Untracked files count.** A file an agent wrote and never staged is
//! exactly the file a person is looking for, and a list that showed
//! only tracked changes would report a new module as nothing at all.
//!
//! **A fence does not move the head.** `checkpoint::wave_pre` files its
//! commit under `refs/sprawling/`, so the head is the base commit or
//! the last landing, and a comparison against it would report every
//! wave since as uncommitted. The caller therefore names the fence it
//! wants compared against; the head is the fallback for a city that has
//! fenced nothing.

use std::path::Path;

use kernel::{FileChange, GitOid};

use crate::changes::collect;
use crate::error::MemoryError;

/// How far a branch has run from the upstream it tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drift {
    pub ahead: u64,
    pub behind: u64,
}

/// Where a repository stands right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingStatus {
    /// The branch checked out; `None` on a detached head, which is a
    /// state a person can be in and one a name cannot describe.
    pub branch: Option<String>,
    /// `None` when the branch tracks no upstream — which is not the
    /// same fact as being level with one.
    pub drift: Option<Drift>,
    /// Files that differ from the head commit, untracked ones included,
    /// in path order.
    pub files: Vec<FileChange>,
}

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> MemoryError {
    move |err| MemoryError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

/// What stands between `base` and the disk under `scope`, and where the
/// repository itself stands.
///
/// `scope` is a repository-relative directory: a building is a directory
/// and the question a building's page asks is about its own files, so
/// filtering here costs one pathspec rather than a second pass over the
/// answer. `base` is the fence to compare against; absent falls back to
/// the head, which is what a city that has fenced nothing has.
///
/// A repository with no commit at all answers every file as added
/// rather than failing: a city initialised this minute has a working
/// tree and no head, and "none of this is fenced" is what a reader
/// needs to be told.
///
/// # Errors
/// Propagates whatever opening the repository, reading its head,
/// finding `base`, or walking the difference reports.
pub fn working_status(
    city_root: &Path,
    scope: Option<&str>,
    base: Option<GitOid>,
) -> Result<WorkingStatus, MemoryError> {
    let repo = git2::Repository::open(city_root).map_err(git_err("open the city repository"))?;
    let head = match repo.head() {
        Ok(found) => Some(found),
        // An unborn branch is a repository with no commit on it yet.
        Err(err) if err.code() == git2::ErrorCode::UnbornBranch => None,
        Err(err) => return Err(git_err("read the repository head")(err)),
    };
    let branch = head
        .as_ref()
        .filter(|found| found.is_branch())
        .and_then(|found| found.shorthand().ok())
        .map(str::to_owned);
    let commit = match base {
        Some(oid) => {
            let parsed =
                git2::Oid::from_str(&oid.to_string()).map_err(git_err("parse a checkpoint"))?;
            Some(
                repo.find_commit(parsed)
                    .map_err(git_err("find a checkpoint"))?,
            )
        }
        None => match &head {
            Some(found) => Some(
                found
                    .peel_to_commit()
                    .map_err(git_err("find the head commit"))?,
            ),
            None => None,
        },
    };
    let tree = match &commit {
        Some(found) => Some(found.tree().map_err(git_err("read a checkpoint tree"))?),
        None => None,
    };
    let mut options = git2::DiffOptions::new();
    options.context_lines(0);
    options.include_untracked(true);
    options.recurse_untracked_dirs(true);
    if let Some(scope) = scope {
        options.pathspec(scope);
    }
    let mut diff = repo
        .diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut options))
        .map_err(git_err("compare the working tree with the head"))?;
    diff.find_similar(None)
        .map_err(git_err("look for renames"))?;
    Ok(WorkingStatus {
        branch: branch.clone(),
        drift: drift_of(&repo, branch.as_deref())?,
        files: collect(&diff)?,
    })
}

/// How far the named branch has run from its upstream.
///
/// `None` for a detached head and for a branch nobody set an upstream
/// on; a branch whose upstream was deleted behind its back reads the
/// same way, because what a reader can act on is identical.
fn drift_of(repo: &git2::Repository, branch: Option<&str>) -> Result<Option<Drift>, MemoryError> {
    let Some(name) = branch else {
        return Ok(None);
    };
    let Ok(local) = repo.find_branch(name, git2::BranchType::Local) else {
        return Ok(None);
    };
    let Ok(upstream) = local.upstream() else {
        return Ok(None);
    };
    let (Some(here), Some(there)) = (local.get().target(), upstream.get().target()) else {
        return Ok(None);
    };
    let (ahead, behind) = repo
        .graph_ahead_behind(here, there)
        .map_err(git_err("measure the drift from the upstream"))?;
    Ok(Some(Drift {
        ahead: u64::try_from(ahead).unwrap_or(u64::MAX),
        behind: u64::try_from(behind).unwrap_or(u64::MAX),
    }))
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
    use kernel::{How, TimeMs};

    use crate::checkpoint::Checkpoint;

    fn oid_of(payload: &kernel::Payload) -> GitOid {
        let raw = serde_json::to_value(payload).unwrap()["oid"]
            .as_str()
            .unwrap()
            .to_owned();
        GitOid::parse(&raw).expect("a checkpoint names a git object")
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn resident() -> crate::checkpoint::Provenance {
        crate::checkpoint::Provenance::new(
            kernel::RunId::CITY,
            kernel::Address::parse("lab/parser").unwrap(),
            kernel::B3Hash::digest(b"a city"),
            crate::checkpoint::ModelChoice {
                id: "test-model".to_owned(),
                effort: None,
            },
        )
    }

    /// The file a person is looking for is the one an agent wrote and
    /// never staged, so an untracked file is a row like any other.
    #[test]
    fn a_file_written_since_the_last_fence_is_listed_untracked_included() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut fence = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &fence
                .wave_pre(&["lab".to_owned()], TimeMs::new(1), &resident())
                .unwrap(),
        );

        write(root, "lab/lex.rs", "one\ntwo\n");
        write(root, "lab/fresh.rs", "new\n");
        let status = working_status(root, Some("lab"), Some(base)).unwrap();
        let paths: Vec<&str> = status.files.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(paths, vec!["lab/fresh.rs", "lab/lex.rs"]);
        assert_eq!(status.files[0].how, How::Added);
        assert_eq!(status.files[1].how, How::Modified);
    }

    /// A building asks about its own files, so another building's work
    /// is not in its answer.
    #[test]
    fn the_scope_keeps_another_buildings_work_out_of_the_answer() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut fence = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &fence
                .wave_pre(&["lab".to_owned()], TimeMs::new(1), &resident())
                .unwrap(),
        );

        write(root, "hall/Notes.md", "elsewhere\n");
        assert!(
            working_status(root, Some("lab"), Some(base))
                .unwrap()
                .files
                .is_empty()
        );
        let whole = working_status(root, None, Some(base)).unwrap();
        assert_eq!(
            whole
                .files
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["hall/Notes.md"]
        );
    }

    /// A branch nobody set an upstream on is not a branch level with
    /// one, and a page that read `0/0` would say the work is pushed.
    #[test]
    fn a_branch_with_no_upstream_reports_no_drift_rather_than_zero() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut fence = Checkpoint::open(root).unwrap();
        fence
            .ensure_base(&["lab".to_owned()], TimeMs::new(1), &resident())
            .unwrap();
        let status = working_status(root, None, None).unwrap();
        assert!(status.branch.is_some(), "{status:?}");
        assert_eq!(status.drift, None);
    }

    /// A city fenced nothing yet still has a working tree, and every
    /// file in it is work no commit holds.
    #[test]
    fn a_city_with_no_commit_reports_its_files_rather_than_failing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        Checkpoint::open(root).unwrap();
        let status = working_status(root, None, None).unwrap();
        assert_eq!(status.branch, None);
        assert_eq!(
            status
                .files
                .iter()
                .map(|row| row.how.clone())
                .collect::<Vec<_>>(),
            vec![How::Added]
        );
    }
}
