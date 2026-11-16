// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::{Cell, RefCell};
use std::path::Path;

use kernel::{FileChange, GitOid, How, Lines};

use crate::error::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Head {
    Commit(GitOid),
    WorkingTree,
}

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> StorageError {
    move |err| StorageError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

pub fn between(
    city_root: &Path,
    base: GitOid,
    head: Head,
) -> Result<Vec<FileChange>, StorageError> {
    let repo = git2::Repository::open(city_root).map_err(git_err("open the city repository"))?;
    let find = |oid: GitOid| -> Result<git2::Tree<'_>, StorageError> {
        let parsed =
            git2::Oid::from_str(&oid.to_string()).map_err(git_err("parse a checkpoint"))?;
        repo.find_commit(parsed)
            .map_err(git_err("find a checkpoint"))?
            .tree()
            .map_err(git_err("read a checkpoint tree"))
    };
    let old = find(base)?;
    let mut options = git2::DiffOptions::new();

    options.context_lines(0);
    let mut diff = match head {
        Head::Commit(oid) => {
            let new = find(oid)?;
            repo.diff_tree_to_tree(Some(&old), Some(&new), Some(&mut options))
        }

        Head::WorkingTree => repo.diff_tree_to_workdir_with_index(Some(&old), Some(&mut options)),
    }
    .map_err(git_err("compare two checkpoints"))?;
    diff.find_similar(None)
        .map_err(git_err("look for renames"))?;
    collect(&diff)
}

pub fn parents_of(
    city_root: &Path,
    oids: &[GitOid],
) -> Result<Vec<Option<Vec<GitOid>>>, StorageError> {
    let repo = git2::Repository::open(city_root).map_err(git_err("open the city repository"))?;
    oids.iter()
        .map(|oid| {
            let parsed =
                git2::Oid::from_str(&oid.to_string()).map_err(git_err("parse a commit"))?;
            let commit = match repo.find_commit(parsed) {
                Ok(commit) => commit,
                Err(absent) if absent.code() == git2::ErrorCode::NotFound => return Ok(None),
                Err(failed) => return Err(git_err("find a commit")(failed)),
            };
            commit
                .parent_ids()
                .map(|id| {
                    let spelled = id.to_string();
                    GitOid::parse(&spelled).ok_or_else(|| StorageError::Checkpoint {
                        op: "read a commit's parents",
                        detail: format!("git named a parent {spelled}, which is not 40 hex digits"),
                    })
                })
                .collect::<Result<Vec<GitOid>, StorageError>>()
                .map(Some)
        })
        .collect()
}

pub(crate) fn collect(diff: &git2::Diff<'_>) -> Result<Vec<FileChange>, StorageError> {
    let mut rows: Vec<FileChange> = Vec::new();
    let stats: Vec<(u32, u32)> = line_counts(diff)?;
    for (at, delta) in diff.deltas().enumerate() {
        let Some(path) = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(Path::to_str)
        else {

            continue;
        };
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "git2::Delta is an upstream open enum; every status we do not name is a change to content"
        )]
        let how = match delta.status() {

            git2::Delta::Added | git2::Delta::Copied | git2::Delta::Untracked => How::Added,
            git2::Delta::Deleted => How::Deleted,
            git2::Delta::Renamed => How::Renamed {
                from: delta
                    .old_file()
                    .path()
                    .and_then(Path::to_str)
                    .unwrap_or_default()
                    .to_owned(),
            },
            _ => How::Modified,
        };

        let binary = delta.new_file().is_binary() || delta.old_file().is_binary();
        let lines = match (binary, stats.get(at)) {
            (false, Some(&(added, removed))) => Lines::Counted { added, removed },
            _ => Lines::Binary,
        };
        rows.push(FileChange {
            path: path.to_owned(),
            how,
            lines,
        });
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(rows)
}

fn line_counts(diff: &git2::Diff<'_>) -> Result<Vec<(u32, u32)>, StorageError> {

    let counts = RefCell::new(vec![(0u32, 0u32); diff.deltas().len()]);
    let at = Cell::new(0usize);
    let started = Cell::new(false);
    diff.foreach(
        &mut |_, _| {

            if started.replace(true) {
                at.set(at.get().saturating_add(1));
            }
            true
        },
        None,
        None,
        Some(&mut |_, _, line| {
            if let Ok(mut rows) = counts.try_borrow_mut()
                && let Some(row) = rows.get_mut(at.get())
            {
                match line.origin() {
                    '+' => row.0 = row.0.saturating_add(1),
                    '-' => row.1 = row.1.saturating_add(1),
                    _ => {}
                }
            }
            true
        }),
    )
    .map_err(git_err("count the lines that moved"))?;
    Ok(counts.into_inner())
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
    use kernel::TimeMs;

    use crate::checkpoint::Checkpoint;

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

    fn oid_of(payload: &kernel::Payload) -> GitOid {
        let raw = serde_json::to_value(payload).unwrap()["oid"]
            .as_str()
            .unwrap()
            .to_owned();
        GitOid::parse(&raw).expect("a checkpoint names a git object")
    }

    #[test]
    fn two_checkpoints_report_the_files_between_them_and_how_much_moved() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\ntwo\nthree\n");
        write(root, "lab/keep.rs", "unchanged\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(1_000), &resident())
                .unwrap(),
        );

        write(root, "lab/lex.rs", "one\ntwo\nthree\nfour\nfive\n");
        write(root, "lab/new.rs", "fresh\n");
        std::fs::remove_file(root.join("lab/keep.rs")).unwrap();
        let head = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(2_000), &resident())
                .unwrap(),
        );

        let rows = between(root, base, Head::Commit(head)).unwrap();
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(paths, vec!["lab/keep.rs", "lab/lex.rs", "lab/new.rs"]);

        assert_eq!(rows[0].how, How::Deleted);
        assert_eq!(rows[1].how, How::Modified);
        assert_eq!(rows[2].how, How::Added);
        assert_eq!(
            rows[1].lines,
            Lines::Counted {
                added: 2,
                removed: 0
            },
            "two lines were appended to the lexer"
        );
    }

    #[test]
    fn two_identical_checkpoints_report_nothing_rather_than_failing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(1), &resident())
                .unwrap(),
        );
        let head = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(2), &resident())
                .unwrap(),
        );
        assert!(between(root, base, Head::Commit(head)).unwrap().is_empty());
    }

    #[test]
    fn work_not_yet_checkpointed_still_counts_against_the_working_tree() {
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
        let rows = between(root, base, Head::WorkingTree).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "lab/lex.rs");
        assert_eq!(
            rows[0].lines,
            Lines::Counted {
                added: 1,
                removed: 0
            }
        );
    }

    #[test]
    fn a_binary_file_says_so_rather_than_reporting_nothing_moved() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("lab")).unwrap();
        std::fs::write(root.join("lab/blob.bin"), [0u8, 1, 2, 0, 3]).unwrap();
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(1), &resident())
                .unwrap(),
        );

        std::fs::write(root.join("lab/blob.bin"), [0u8, 9, 9, 0, 7, 7]).unwrap();
        let rows = between(root, base, Head::WorkingTree).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lines, Lines::Binary, "{:?}", rows[0]);
    }

    #[test]
    fn a_checkpoint_this_city_does_not_hold_is_refused_rather_than_read_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(1), &resident())
                .unwrap(),
        );
        let stranger = GitOid::from_bytes([7u8; 20]);
        assert!(between(root, stranger, Head::Commit(base)).is_err());
    }
}
