// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use kernel::TimeMs;
use kernel::event::record::{CheckpointFinding, Commit, SecretEgressBlocked};
use kernel::secret::{SecretSpan, scan};

use crate::error::StorageError;

use super::commit::{Checkpoint, CommitDetails, commit_record, git_err};
use super::opening::{HeadMove, commit_refused};
use super::provenance::Provenance;

const GITLINK: u32 = 0o160_000;

struct StagedBlob<'a> {
    id: git2::Oid,

    path: &'a [u8],
    index: &'a git2::Index,
}

#[derive(Default)]
struct ScanFindings {
    refused: Vec<CheckpointFinding>,
    withheld: Vec<CheckpointFinding>,
}

fn checkpoint_size(value: usize, path: &str) -> Result<u64, StorageError> {
    u64::try_from(value).map_err(|_| StorageError::Checkpoint {
        op: "record a checkpoint scan",
        detail: format!("{path}: a finding's offset or length exceeds u64"),
    })
}

fn names_staged_path(content: &[u8], span: &SecretSpan, index: &git2::Index) -> bool {
    let Some(span_end) = span.start.checked_add(span.len) else {
        return false;
    };
    if span_end > content.len() {
        return false;
    }
    let mut first = span.start;
    while let Some(before) = first.checked_sub(1) {
        if !content.get(before).copied().is_some_and(path_byte) {
            break;
        }
        first = before;
    }
    let mut end = span_end;
    while content.get(end).copied().is_some_and(path_byte) {
        end = end.saturating_add(1);
    }

    if content.get(first.saturating_sub(1)) == Some(&b'\\') || content.get(end) == Some(&b'\\') {
        return false;
    }
    loop {
        let named = content
            .get(first..end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|path| {
                path.split('/').all(|segment| {
                    !segment.is_empty()
                        && segment.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                        })
                }) && index.get_path(std::path::Path::new(path), 0).is_some()
            });
        if named {
            return true;
        }

        let last = end.checked_sub(1).and_then(|at| content.get(at));
        if end <= span_end || last != Some(&b'.') {
            return false;
        }
        end = end.saturating_sub(1);
    }
}

fn path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/')
}

mod index_lock;
pub(crate) mod pathspec;
mod stage_filter;
pub(super) use index_lock::write_index;
use stage_filter::{StageFilter, workdir};

pub(crate) struct CommitPlan<'a> {
    pub(crate) t: TimeMs,
    pub(crate) of: &'a Provenance,
    pub(crate) subject: &'a str,
    pub(crate) head: HeadMove,
}

impl Checkpoint {

    pub fn scan_staged(
        &mut self,
        scopes: &[String],
    ) -> Result<Vec<CheckpointFinding>, StorageError> {
        let index = self.repo.index().map_err(git_err("read index"))?;
        let mut findings = ScanFindings::default();
        match self.last_tree()? {

            None => {
                for entry in index.iter() {
                    if entry.mode != GITLINK {
                        self.scan_blob(
                            StagedBlob {
                                id: entry.id,
                                path: &entry.path,
                                index: &index,
                            },
                            &mut findings,
                        )?;
                    }
                }
            }

            Some(tree) => {
                let diff = self
                    .repo
                    .diff_tree_to_index(Some(&tree), Some(&index), None)
                    .map_err(git_err("diff checkpoint against the staged tree"))?;
                for delta in diff.deltas() {
                    let staged = delta.new_file();

                    if staged.id().is_zero() || staged.mode() == git2::FileMode::Commit {
                        continue;
                    }
                    self.scan_blob(
                        StagedBlob {
                            id: staged.id(),
                            path: staged.path_bytes().unwrap_or_default(),
                            index: &index,
                        },
                        &mut findings,
                    )?;
                }
            }
        }
        let ScanFindings { refused, withheld } = findings;
        if refused.is_empty() {
            return Ok(withheld);
        }

        let mut all = refused;
        all.extend(withheld);
        Err(StorageError::SecretEgress(SecretEgressBlocked::checkpoint(
            scopes.to_vec(),
            all,
        )))
    }

    fn scan_blob(
        &self,
        staged: StagedBlob<'_>,
        findings: &mut ScanFindings,
    ) -> Result<(), StorageError> {
        let path = String::from_utf8_lossy(staged.path);
        let blob = self
            .repo
            .find_blob(staged.id)
            .map_err(|err| StorageError::Checkpoint {
                op: "scan a staged file",
                detail: format!("{path}: {}", err.message()),
            })?;

        let allowed =
            std::str::from_utf8(staged.path).is_ok_and(|path| self.entropy_allow.allows(path));
        let content = blob.content();
        let mut cursor = 0usize;
        let mut line = 1u64;
        for span in scan(content) {
            let newlines = content
                .iter()
                .skip(cursor)
                .take(span.start.saturating_sub(cursor))
                .filter(|byte| **byte == b'\n')
                .count();
            line = line.saturating_add(u64::try_from(newlines).unwrap_or(u64::MAX));
            cursor = span.start;
            let finding = CheckpointFinding {
                path: path.to_string(),
                offset: checkpoint_size(span.start, &path)?,
                len: checkpoint_size(span.len, &path)?,
                line,
                reason: span.reason().to_owned(),
            };
            if span.provider.is_some() {
                findings.refused.push(finding);
            } else if !allowed && !names_staged_path(content, &span, staged.index) {
                findings.withheld.push(finding);
            }
        }
        Ok(())
    }

    fn last_tree(&self) -> Result<Option<git2::Tree<'_>>, StorageError> {
        let commit = match self.last {
            Some(oid) => self
                .repo
                .find_commit(oid)
                .map_err(git_err("read the last checkpoint"))?,
            None => match Self::head_commit(&self.repo)? {
                Some(commit) => commit,
                None => return Ok(None),
            },
        };
        commit
            .tree()
            .map(Some)
            .map_err(git_err("read the last checkpoint's tree"))
    }

    pub(super) fn head_commit(
        repo: &git2::Repository,
    ) -> Result<Option<git2::Commit<'_>>, StorageError> {
        match repo.head() {
            Ok(head) => head
                .peel_to_commit()
                .map(Some)
                .map_err(git_err("read HEAD")),
            Err(err)
                if matches!(
                    err.code(),
                    git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
                ) =>
            {
                Ok(None)
            }
            Err(err) => Err(git_err("read HEAD")(err)),
        }
    }

    pub(crate) fn stage_scopes(&mut self, scopes: &[String]) -> Result<Vec<String>, StorageError> {
        let files = self.stage_scopes_held(scopes)?;
        write_index(&mut self.repo.index().map_err(git_err("read index"))?)?;
        Ok(files)
    }

    pub(crate) fn stage_scopes_held(
        &mut self,
        scopes: &[String],
    ) -> Result<Vec<String>, StorageError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let found: BTreeMap<Vec<u8>, git2::Oid> =
            index.iter().map(|entry| (entry.path, entry.id)).collect();
        let root = workdir(&self.repo)?;
        let (files, prefixes): (Vec<&String>, Vec<&String>) = scopes
            .iter()
            .partition(|scope| names_a_file(root, &index, scope));
        let mut filter = StageFilter::new(root);
        for file in files {
            self.stage_file(&mut index, &mut filter, std::path::Path::new(file.as_str()))?;
        }
        if scopes.is_empty() || !prefixes.is_empty() {
            let patterns = pathspec::of(&prefixes.into_iter().cloned().collect::<Vec<_>>());
            let specs: Vec<&str> = patterns.iter().map(String::as_str).collect();
            let mut admit = |path: &std::path::Path, _matched: &[u8]| -> i32 { filter.admit(path) };
            index
                .add_all(
                    specs.iter(),
                    git2::IndexAddOption::DEFAULT,
                    Some(&mut admit),
                )
                .map_err(git_err("stage scope"))?;
        }
        filter.refused()?;
        let mut nested = filter.nested().to_vec();
        nested.sort();
        nested.dedup();
        for root in nested {

            if index.get_path(&root, 0).is_some() {
                index.remove_path(&root).map_err(git_err("stage scope"))?;
            }
            self.stage_nested(&mut index, &mut filter, &root)?;
        }
        filter.refused()?;
        let mut changed: BTreeSet<Vec<u8>> = BTreeSet::new();
        let mut present = 0usize;
        for entry in index.iter() {
            let unchanged = match found.get(&entry.path) {
                Some(id) => {
                    present = present.saturating_add(1);
                    *id == entry.id
                }
                None => false,
            };
            if !unchanged {
                changed.insert(entry.path);
            }
        }

        if present < found.len() {
            let staged: BTreeSet<Vec<u8>> = index.iter().map(|entry| entry.path).collect();
            changed.extend(found.into_keys().filter(|path| !staged.contains(path)));
        }
        Ok(changed
            .into_iter()
            .map(|path| String::from_utf8_lossy(&path).into_owned())
            .collect())
    }

    fn stage_file(
        &self,
        index: &mut git2::Index,
        filter: &mut StageFilter,
        file: &std::path::Path,
    ) -> Result<(), StorageError> {
        let ignored = self
            .repo
            .is_path_ignored(file)
            .map_err(git_err("stage scope"))?;
        if filter.admit(file) != 0 || (ignored && index.get_path(file, 0).is_none()) {
            return Ok(());
        }
        if workdir(&self.repo)?.join(file).is_file() {
            index.add_path(file)
        } else {
            index.remove_path(file)
        }
        .map_err(git_err("stage scope"))
    }

    fn stage_nested(
        &self,
        index: &mut git2::Index,
        filter: &mut StageFilter,
        relative: &std::path::Path,
    ) -> Result<(), StorageError> {
        let directory = workdir(&self.repo)?.join(relative);
        let refused = |err: std::io::Error| StorageError::Checkpoint {
            op: "stage a nested repository",
            detail: format!("{}: {err}", relative.display()),
        };
        let mut entries = std::fs::read_dir(directory)
            .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
            .map_err(refused)?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = relative.join(entry.file_name());
            if entry.file_type().map_err(refused)?.is_dir() {
                if filter.admit_directory(&path) == 0 {
                    self.stage_nested(index, filter, &path)?;
                }
            } else {
                self.stage_file(index, filter, &path)?;
            }
        }
        Ok(())
    }

    pub(crate) fn commit(&mut self, plan: &CommitPlan<'_>) -> Result<git2::Oid, StorageError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let tree_oid = index.write_tree().map_err(git_err("write tree"))?;
        let tree = self
            .repo
            .find_tree(tree_oid)
            .map_err(git_err("find staged tree"))?;
        let seconds = i64::try_from(plan.t.value().saturating_div(1000)).map_err(|_| {
            StorageError::Checkpoint {
                op: "stamp the commit",
                detail: format!("{} ms is past what git can date", plan.t.value()),
            }
        })?;
        let when = git2::Time::new(seconds, 0);
        let email = plan.of.email();
        let signature = git2::Signature::new(plan.of.actor().as_str(), &email, &when)
            .map_err(git_err("build signature"))?;
        let (update, parents): (_, Vec<git2::Commit>) = match plan.head {
            HeadMove::Leave => (None, Self::head_commit(&self.repo)?.into_iter().collect()),
            HeadMove::Advance => (
                Some("HEAD"),
                Self::head_commit(&self.repo)?.into_iter().collect(),
            ),
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        let full_message = format!("{}\n\n{}", plan.subject, plan.of.trailers());
        let oid = self
            .repo
            .commit(
                update,
                &signature,
                &signature,
                &full_message,
                &tree,
                &parent_refs,
            )
            .map_err(|err| commit_refused(update, err))?;
        self.last = Some(oid);
        Ok(oid)
    }

    pub fn land(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        subject: &str,
    ) -> Result<Commit, StorageError> {
        let _head_moves = super::opening::moving_head()?;
        let files = self.stage_scopes(scopes)?;

        let withheld = self.scan_staged(scopes)?;
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject,
            head: HeadMove::Advance,
        })?;
        commit_record(
            oid,
            CommitDetails {
                of,
                scopes,
                files,
                withheld,
            },
        )
    }
}

fn names_a_file(root: &std::path::Path, index: &git2::Index, scope: &str) -> bool {
    match std::fs::symlink_metadata(root.join(scope)) {
        Ok(meta) => meta.is_file(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            index.get_path(std::path::Path::new(scope), 0).is_some()
        }

        Err(_) => false,
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
mod tests;
