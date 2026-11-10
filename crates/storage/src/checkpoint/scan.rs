// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint scans: staged secrets and scoped commits.

use std::collections::{BTreeMap, BTreeSet};

use kernel::TimeMs;
use kernel::event::record::{CheckpointFinding, Commit, SecretEgressBlocked};
use kernel::secret::{SecretSpan, scan};

use crate::error::StorageError;

use super::commit::{Checkpoint, CommitDetails, commit_record, git_err};
use super::opening::{HeadMove, commit_refused};
use super::provenance::Provenance;

/// The index mode of a submodule entry: a commit of another repository,
/// which this repository's object database does not hold.
const GITLINK: u32 = 0o160_000;

/// One staged blob and the index that proves whether a quoted path exists.
struct StagedBlob<'a> {
    id: git2::Oid,
    /// The path as the repository records it, never a lossy respelling: a
    /// finding reports the lossy text, but an `Address` cannot spell any
    /// bytes but UTF-8, so only a valid path may match an allowlist entry.
    path: &'a [u8],
    index: &'a git2::Index,
}

/// Findings partitioned by the action their evidence justifies.
#[derive(Default)]
struct ScanFindings {
    refused: Vec<CheckpointFinding>,
    withheld: Vec<CheckpointFinding>,
}

/// Converts a scanner's native offset or length into the Ledger's integer
/// shape without truncating it.
fn checkpoint_size(value: usize, path: &str) -> Result<u64, StorageError> {
    u64::try_from(value).map_err(|_| StorageError::Checkpoint {
        op: "record a checkpoint scan",
        detail: format!("{path}: a finding's offset or length exceeds u64"),
    })
}

/// Whether the matched text is a relative path this index actually stages.
/// This is deliberately a small byte predicate rather than a regex: every
/// segment is non-empty and uses only ASCII alphanumerics, `.`, `_` or `-`.
///
/// The scanner stops the entropy run at the first byte no token charset
/// holds, and `.` is one of those, so a quoted `docs/glossary.md` arrives as
/// `docs/glossary`: the path is the token around the span, not the span itself.
/// A trailing `.` may also be the sentence's, so the longest spelling that
/// names a staged file wins.
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
    // A backslash just outside the token belongs to the same reference on
    // one platform and is a legal file-name byte on another. Reading the
    // token as the whole reference would exempt `other\docs/glossary.md` on the
    // strength of `docs/glossary.md`, so the backslash boundary is refused.
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
        // Only a dot beyond the span can be the sentence's.
        let last = end.checked_sub(1).and_then(|at| content.get(at));
        if end <= span_end || last != Some(&b'.') {
            return false;
        }
        end = end.saturating_sub(1);
    }
}

/// The bytes a relative path this repository could stage is spelled with:
/// the segment alphabet plus the `/` between segments.
fn path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/')
}

mod index_lock;
pub(crate) mod pathspec;
mod stage_filter;
pub(super) use index_lock::write_index;
use stage_filter::{StageFilter, workdir};

/// One commit, decided before anything is written.
///
/// Four values that are meaningless apart — a subject without the
/// provenance that signs it, an instant without the commit it stamps —
/// so they arrive as one value rather than as four parameters a caller
/// can put in the wrong order.
pub(crate) struct CommitPlan<'a> {
    pub(crate) t: TimeMs,
    pub(crate) of: &'a Provenance,
    pub(crate) subject: &'a str,
    pub(crate) head: HeadMove,
}

impl Checkpoint {
    /// Scans what this checkpoint would newly write into the tree.
    /// Known provider shapes refuse; other entropy findings are returned for
    /// the committed record. Positions and reasons only, never matched bytes.
    pub fn scan_staged(
        &mut self,
        scopes: &[String],
    ) -> Result<Vec<CheckpointFinding>, StorageError> {
        let index = self.repo.index().map_err(git_err("read index"))?;
        let mut findings = ScanFindings::default();
        match self.last_tree()? {
            // No previous tree to compare against: this checkpoint is
            // writing all of it, so all of it is read.
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
            // git is asked what changed, the same way `wave_post` asks it
            // what went missing, so "what counts as a change" has one
            // answer in this module rather than two.
            Some(tree) => {
                let diff = self
                    .repo
                    .diff_tree_to_index(Some(&tree), Some(&index), None)
                    .map_err(git_err("diff checkpoint against the staged tree"))?;
                for delta in diff.deltas() {
                    let staged = delta.new_file();
                    // A deletion names no new content, and a submodule's
                    // commit is not this repository's to read.
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
        // The refusal carries every finding of the same scan: a person
        // reading `sprawling secrets` should not have to wait for a later
        // checkpoint to learn about the entropy hits this one also met.
        let mut all = refused;
        all.extend(withheld);
        Err(StorageError::SecretEgress(SecretEgressBlocked::checkpoint(
            scopes.to_vec(),
            all,
        )))
    }

    /// One blob against the secret shapes. The line cursor follows the
    /// scanner's ordered starts, so locating every hit still reads the blob
    /// once rather than once per hit.
    ///
    /// # Errors
    /// A blob the object database will not hand back: a byte that enters
    /// the tree unread breaks "every byte that entered the tree was scanned
    /// once" (`crates/storage/spec/Checkpoint.lean` §8-8), so the checkpoint
    /// is refused rather than the blob skipped.
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
        // An allowlist entry is an `Address`, and an `Address` is UTF-8:
        // a path the repository spells in other bytes cannot be the one the
        // User entered, so it is never silently exempt.
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

    /// The tree the last checkpoint committed, or `None` when this city
    /// has never been checkpointed. An unborn HEAD is a state, not a failure -
    /// it is what an empty repository looks like.
    ///
    /// A wave checkpoint does not move HEAD, so the last
    /// checkpoint is remembered here rather than read off the branch. A
    /// process that has just opened this repository remembers nothing
    /// and falls back to HEAD, which is older: the scan then re-reads
    /// blobs it has already cleared, which costs time and gives up no
    /// safety.
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

    /// The commit HEAD names, or `None` when the city has none yet.
    ///
    /// Only an unborn branch and a missing reference mean "none yet";
    /// anything else that stops HEAD being read is an error, because
    /// reading it as "none" would turn the next checkpoint into a parentless
    /// root commit cut off from the history before it (`crates/storage/spec/Checkpoint.lean` §8-8).
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

    /// Stages every file under `scope`, including deletions. Paths
    /// outside the scope are never touched — the write domain is the
    /// boundary, and a wider `add` would stage what the Run never held.
    ///
    /// One `add_all`, which stages removals as well: libgit2 walks the
    /// index against the working tree once and skips each unchanged file
    /// by its stat data, and a second `update_all` walk repaid nothing.
    ///
    /// **A session slice is never staged.** It is a disposable
    /// projection the city's own accounting thread appends to while the
    /// wave runs, and a checkpoint that staged it would ask git to read a
    /// workdir file it believes it already knows; a file still growing
    /// under an open handle makes that read refuse the whole wave
    /// (`crates/storage/Spec.lean` §8-8, §8-24). Nor does one ever stage protected
    /// metadata or an alias - [`StageFilter`] owns that rule.
    ///
    /// Returns the paths whose staged blob this call added, changed or
    /// removed, in byte order: the difference between the index it found
    /// and the index it wrote, which is what the checkpoint touched and not
    /// what the city holds.
    pub(crate) fn stage_scopes(&mut self, scopes: &[String]) -> Result<Vec<String>, StorageError> {
        let files = self.stage_scopes_held(scopes)?;
        write_index(&mut self.repo.index().map_err(git_err("read index"))?)?;
        Ok(files)
    }

    /// Stages `scopes` into the repository's index in memory and leaves
    /// the file on disk as it was, for a caller whose objects are not on
    /// disk yet: an index written before them names blobs a failure
    /// would drop, and later checkpoints skip those entries by their stat data.
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
            // A checkpoint written by an older build may hold a gitlink at
            // this root. Its files replace that pointer; the two shapes
            // cannot occupy the same tree path.
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
        // Only a removal leaves a found path unmatched, so the second
        // pass is paid by the waves that deleted something.
        if present < found.len() {
            let staged: BTreeSet<Vec<u8>> = index.iter().map(|entry| entry.path).collect();
            changed.extend(found.into_keys().filter(|path| !staged.contains(path)));
        }
        Ok(changed
            .into_iter()
            .map(|path| String::from_utf8_lossy(&path).into_owned())
            .collect())
    }

    /// Stages one file the wave wrote, or its removal, by its literal
    /// path: a pathspec walk would cost the whole domain for one file.
    /// The same admission and the same ignore rule as the walk.
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

    /// Stages the ordinary files below a nested repository root. The outer
    /// `add_all` skips that root; this walk supplies its contents without
    /// following a directory link or admitting protected metadata.
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

    /// Commits the index at the injected time. An unchanged tree still
    /// commits: a rebuildable chain is worth more than a saved object.
    ///
    /// The signature is the session's, not a fixed machine identity: the
    /// author is the resident's address at a mailbox naming the city,
    /// and the five `Sprawling-*` trailers say which run, which model
    /// and which effort produced it.
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

    /// Puts this tree on the branch, under the session that produced it.
    ///
    /// This is what a reviewing run does when it offers its work: the
    /// wave checkpoints behind it are dangling commits nobody merges, and
    /// what a verifier judges has to be a commit on the run's own
    /// branch. Time stays a parameter here as everywhere else — the
    /// signature carries the injected instant, so the same script lands
    /// the same oid.
    ///
    /// Only `scopes` is staged, as a checkpoint stages it: a kept tree is
    /// checked out again over its scope alone, so its files outside the
    /// scope may trail the branch, and staging them would take back
    /// what the trunk changed there (`crates/storage/spec/Worktree.lean` §8-9). Entries outside
    /// the scope go into the commit as the index holds them. No scope
    /// stages the whole tree except the reserved subtree, whose rule
    /// [`StageFilter`] owns.
    ///
    /// # Errors
    /// Propagates staging, the staged-secret scan, and the commit.
    ///
    /// The record it answers with is the `checkpoint_committed` value for the
    /// commit it wrote, its `files` and its findings included: a run that
    /// wrote after its last wave checkpoint has its work committed here, and
    /// a caller that dropped this record would drop the only account of what
    /// this landing scanned.
    pub fn land(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        subject: &str,
    ) -> Result<Commit, StorageError> {
        let _head_moves = super::opening::moving_head()?;
        let files = self.stage_scopes(scopes)?;
        // The run's closing wave checkpoint already recorded the entropy
        // findings of what it scanned; this landing's own scan may see more,
        // and its record is where they live.
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

/// Whether a scope names one file rather than a prefix: a file in the
/// working tree, or a path the index holds as a file and the tree no
/// longer has.
fn names_a_file(root: &std::path::Path, index: &git2::Index, scope: &str) -> bool {
    match std::fs::symlink_metadata(root.join(scope)) {
        Ok(meta) => meta.is_file(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            index.get_path(std::path::Path::new(scope), 0).is_some()
        }
        // Any other failure is left to the walk, which reports it.
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
