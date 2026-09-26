// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint scans: staged secrets and scoped commits.

use std::collections::{BTreeMap, BTreeSet};

use kernel::TimeMs;
use kernel::secret::scan;

use crate::error::MemoryError;

use super::fence::{Checkpoint, git_err};
use super::provenance::Provenance;

mod stage_filter;
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
    /// Whether the branch follows this commit. A wave fence says no: it
    /// is filed under its own reference so a person's `git log` does not
    /// grow a line per tool wave. A base commit and a landing
    /// say yes, because a worktree branches from a branch and offered
    /// work has to be on one.
    pub(crate) onto_head: bool,
}

impl Checkpoint {
    /// Scans what this checkpoint would newly write into the tree.
    /// Reports how many shapes matched and where, never what matched.
    pub fn scan_staged(&mut self) -> Result<(), MemoryError> {
        let index = self.repo.index().map_err(git_err("read index"))?;
        let mut hits: Vec<String> = Vec::new();
        match self.last_tree()? {
            // No previous tree to compare against: this checkpoint is
            // writing all of it, so all of it is read.
            None => {
                for entry in index.iter() {
                    self.scan_blob(entry.id, &String::from_utf8_lossy(&entry.path), &mut hits);
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
                    let Some(path) = staged.path().and_then(|p| p.to_str()) else {
                        continue;
                    };
                    self.scan_blob(staged.id(), &path.replace('\\', "/"), &mut hits);
                }
            }
        }
        if hits.is_empty() {
            return Ok(());
        }
        Err(MemoryError::SecretEgress {
            locations: hits.join(" "),
        })
    }

    /// One blob against the secret shapes, appending `path:start+len` per
    /// match. An id the object database will not hand back as a blob is
    /// skipped: a deletion names no new content, and a submodule is not
    /// this repository's to read.
    fn scan_blob(&self, id: git2::Oid, path: &str, hits: &mut Vec<String>) {
        let Ok(blob) = self.repo.find_blob(id) else {
            return;
        };
        for span in scan(blob.content()) {
            hits.push(format!("{path}:{}+{}", span.start, span.len));
        }
    }

    /// The tree the last checkpoint committed, or `None` when this city
    /// has never been fenced. An unborn HEAD is a state, not a failure -
    /// it is what an empty repository looks like.
    ///
    /// A wave fence does not move HEAD, so the last
    /// checkpoint is remembered here rather than read off the branch. A
    /// process that has just opened this repository remembers nothing
    /// and falls back to HEAD, which is older: the scan then re-reads
    /// blobs it has already cleared, which costs time and gives up no
    /// safety.
    fn last_tree(&self) -> Result<Option<git2::Tree<'_>>, MemoryError> {
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
    /// reading it as "none" would turn the next fence into a parentless
    /// root commit cut off from the history before it (memory-SPEC 8-17).
    fn head_commit(repo: &git2::Repository) -> Result<Option<git2::Commit<'_>>, MemoryError> {
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
    /// **A session slice is never staged.** It is a disposable
    /// projection the city's own accounting thread appends to while the
    /// wave runs, and a fence that staged it would ask git to read a
    /// workdir file it believes it already knows; a file still growing
    /// under an open handle makes that read refuse the whole wave
    /// (memory-SPEC 8-8, 8-24). Nor does one ever stage protected
    /// metadata or an alias - [`StageFilter`] owns that rule.
    ///
    /// Returns the paths whose staged blob this call added, changed or
    /// removed, in byte order: the difference between the index it found
    /// and the index it wrote, which is what the fence touched and not
    /// what the city holds.
    pub(crate) fn stage_scopes(&mut self, scopes: &[String]) -> Result<Vec<String>, MemoryError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let found: BTreeMap<Vec<u8>, git2::Oid> =
            index.iter().map(|entry| (entry.path, entry.id)).collect();
        let patterns = Self::pathspecs(scopes);
        let specs: Vec<&str> = patterns.iter().map(String::as_str).collect();
        let mut filter = StageFilter::new(workdir(&self.repo)?);
        {
            let mut admit = |path: &std::path::Path, _matched: &[u8]| -> i32 { filter.admit(path) };
            index
                .add_all(
                    specs.iter(),
                    git2::IndexAddOption::DEFAULT,
                    Some(&mut admit),
                )
                .map_err(git_err("stage scope"))?;
            index
                .update_all(specs.iter(), Some(&mut admit))
                .map_err(git_err("stage deletions"))?;
        }
        filter.refused()?;
        write_index(&mut index)?;
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

    /// One git pathspec per prefix the run may write under.
    ///
    /// Several, because a write domain is a set: a building's own
    /// subtree plus whatever else its `RULES.toml` declares. Staging
    /// one of them and judging against all of them is what left files a
    /// run legitimately wrote outside every checkpoint.
    ///
    /// No prefixes at all means the whole tree rather than nothing: the
    /// caller that passes an empty set is the base fence, which has no
    /// resident to take a domain from.
    fn pathspecs(scopes: &[String]) -> Vec<String> {
        if scopes.is_empty() {
            return vec!["*".to_owned()];
        }
        scopes
            .iter()
            .map(|scope| {
                if scope.is_empty() || scope == "." {
                    "*".to_owned()
                } else {
                    format!("{}/*", scope.trim_end_matches('/'))
                }
            })
            .collect()
    }

    /// Stages the whole working tree except the reserved subtree,
    /// deletions included.
    fn stage_tree(&mut self) -> Result<(), MemoryError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let mut filter = StageFilter::new(workdir(&self.repo)?);
        {
            let mut admit = |path: &std::path::Path, _matched: &[u8]| -> i32 { filter.admit(path) };
            index
                .add_all(["*"], git2::IndexAddOption::DEFAULT, Some(&mut admit))
                .map_err(git_err("stage the tree"))?;
            index
                .update_all(["*"], Some(&mut admit))
                .map_err(git_err("stage deletions"))?;
        }
        filter.refused()?;
        write_index(&mut index)
    }

    /// Commits the index at the injected time. An unchanged tree still
    /// commits: a rebuildable chain is worth more than a saved object.
    ///
    /// The signature is the session's, not a fixed machine identity: the
    /// author is the resident's address at a mailbox naming the city,
    /// and the five `Sprawling-*` trailers say which run, which model
    /// and which effort produced it.
    pub(crate) fn commit(&mut self, plan: &CommitPlan<'_>) -> Result<git2::Oid, MemoryError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let tree_oid = index.write_tree().map_err(git_err("write tree"))?;
        let tree = self
            .repo
            .find_tree(tree_oid)
            .map_err(git_err("find staged tree"))?;
        let seconds = i64::try_from(plan.t.value().saturating_div(1000)).map_err(|_| {
            MemoryError::Checkpoint {
                op: "stamp the commit",
                detail: format!("{} ms is past what git can date", plan.t.value()),
            }
        })?;
        let when = git2::Time::new(seconds, 0);
        let email = plan.of.email();
        let signature = git2::Signature::new(plan.of.actor().as_str(), &email, &when)
            .map_err(git_err("build signature"))?;
        let parents: Vec<git2::Commit> = Self::head_commit(&self.repo)?.into_iter().collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        let full_message = format!("{}\n\n{}", plan.subject, plan.of.trailers());
        let update = if plan.onto_head { Some("HEAD") } else { None };
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
            .map_err(git_err("commit checkpoint"))?;
        self.last = Some(oid);
        Ok(oid)
    }

    /// Puts this tree on the branch, under the session that produced it.
    ///
    /// This is what a reviewing run does when it offers its work: the
    /// wave fences behind it are dangling commits nobody merges, and
    /// what a verifier judges has to be a commit on the run's own
    /// branch. Time stays a parameter here as everywhere else — the
    /// signature carries the injected instant, so the same script lands
    /// the same oid.
    ///
    /// The whole tree is staged rather than one scope: a run that is
    /// landing is offering the tree it was lent, and that tree is its
    /// own. The reserved subtree is the one exception, for the reason it
    /// is always the exception - what governs a scope is not in any
    /// write domain, and in the city itself it also holds the other
    /// runs' working trees.
    ///
    /// # Errors
    /// Propagates staging, the staged-secret scan, and the commit.
    pub fn land(
        &mut self,
        t: TimeMs,
        of: &Provenance,
        subject: &str,
    ) -> Result<String, MemoryError> {
        self.stage_tree()?;
        self.scan_staged()?;
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject,
            onto_head: true,
        })?;
        Ok(oid.to_string())
    }
}

/// Writes the index, waiting out a lock another run of this city holds.
///
/// **`.git/index.lock` is taken for the length of one write, and two runs
/// of one building stage their scopes at the same time by design** -
/// `assembly::plans::pursuing` drives every node of a ready set at once,
/// in one repository. The loser of that race used to be told "the index
/// is locked; this might be due to a concurrent or crashed process",
/// which is true and useless: the concurrent process is this city, the
/// lock is held for microseconds, and the sentence in front of the person
/// said "retry the wave" while nothing retried it. A run that hit the
/// window ended as cancelled, its node was handed back as though its own
/// done check had failed, and its parent was told nothing it could act
/// on.
///
/// **Bounded, because a lock held by a dead process is a different
/// fact.** Twenty-five milliseconds apart, twenty times: a genuinely
/// stuck lock still surfaces as the refusal it is, and that refusal is
/// what a person can act on.
///
/// Retrying is safe because the index being written is the in-memory one
/// this call built, unchanged by a failed write: the second attempt
/// states the same thing as the first.
fn write_index(index: &mut git2::Index) -> Result<(), MemoryError> {
    const ATTEMPTS: u32 = 20;
    const WAIT: std::time::Duration = std::time::Duration::from_millis(25);
    let mut attempt: u32 = 0;
    loop {
        attempt = attempt.saturating_add(1);
        match index.write() {
            Ok(()) => return Ok(()),
            Err(err) if concurrent(&err) && attempt < ATTEMPTS => {}
            Err(err) => return Err(git_err("write index")(err)),
        }
        std::thread::sleep(WAIT);
    }
}

/// Whether git refused because somebody else holds the index lock.
///
/// libgit2 reports that as an index error whose message names the lock
/// file; anything else - a permission problem, a damaged index - is not a
/// collision and is refused on the first attempt.
fn concurrent(err: &git2::Error) -> bool {
    err.class() == git2::ErrorClass::Index && err.message().contains("index.lock")
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
