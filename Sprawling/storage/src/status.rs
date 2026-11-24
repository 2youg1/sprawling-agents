// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{FileChange, GitOid};

use crate::changes::collect;
use crate::error::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drift {
    pub ahead: u64,
    pub behind: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingStatus {
    pub branch: Option<String>,

    pub drift: Option<Drift>,

    pub files: Vec<FileChange>,
}

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> StorageError {
    move |err| StorageError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

pub fn working_status(
    city_root: &Path,
    scope: Option<&str>,
    base: Option<GitOid>,
) -> Result<WorkingStatus, StorageError> {
    let repo = git2::Repository::open(city_root).map_err(git_err("open the city repository"))?;
    working_status_in(&repo, scope, base)
}

pub(crate) fn working_status_in(
    repo: &git2::Repository,
    scope: Option<&str>,
    base: Option<GitOid>,
) -> Result<WorkingStatus, StorageError> {
    let head = match repo.head() {
        Ok(found) => Some(found),

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
        drift: drift_of(repo, branch.as_deref())?,
        files: collect(&diff)?,
    })
}

fn drift_of(repo: &git2::Repository, branch: Option<&str>) -> Result<Option<Drift>, StorageError> {
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

    #[test]
    fn a_file_written_since_the_last_checkpoint_is_listed_untracked_included() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
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

    #[test]
    fn the_scope_keeps_another_buildings_work_out_of_the_answer() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
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

    #[test]
    fn a_branch_with_no_upstream_reports_no_drift_rather_than_zero() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        checkpoint
            .ensure_base(&["lab".to_owned()], TimeMs::new(1), &resident())
            .unwrap();
        let status = working_status(root, None, None).unwrap();
        assert!(status.branch.is_some(), "{status:?}");
        assert_eq!(status.drift, None);
    }

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
