// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint scans: staged secrets and scoped commits.

use kernel::{TimeMs, scan};

use crate::error::MemoryError;

use super::fence::{Checkpoint, git_err};
use super::provenance::Provenance;

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
    /// grow a line per tool wave (card-2.2). A base commit and a landing
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
    /// Since card-2.2 a wave fence does not move HEAD, so the last
    /// checkpoint is remembered here rather than read off the branch. A
    /// process that has just opened this repository remembers nothing
    /// and falls back to HEAD, which is older: the scan then re-reads
    /// blobs it has already cleared, which costs time and gives up no
    /// safety.
    fn last_tree(&self) -> Result<Option<git2::Tree<'_>>, MemoryError> {
        let oid = match self.last {
            Some(oid) => oid,
            None => match self.repo.head().ok().and_then(|head| head.target()) {
                Some(oid) => oid,
                None => return Ok(None),
            },
        };
        let commit = self
            .repo
            .find_commit(oid)
            .map_err(git_err("read the last checkpoint"))?;
        commit
            .tree()
            .map(Some)
            .map_err(git_err("read the last checkpoint's tree"))
    }

    /// Stages every file under `scope`, including deletions. Paths
    /// outside the scope are never touched — the write domain is the
    /// boundary, and a wider `add` would stage what the Run never held.
    pub(crate) fn stage_scope(&mut self, scope: &str) -> Result<Vec<String>, MemoryError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let pattern = if scope.is_empty() || scope == "." {
            "*".to_owned()
        } else {
            format!("{}/*", scope.trim_end_matches('/'))
        };
        index
            .add_all([&pattern], git2::IndexAddOption::DEFAULT, None)
            .map_err(git_err("stage scope"))?;
        index
            .update_all([&pattern], None)
            .map_err(git_err("stage deletions"))?;
        index.write().map_err(git_err("write index"))?;
        let mut files: Vec<String> = index
            .iter()
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
            .collect();
        files.sort();
        Ok(files)
    }

    /// Stages the whole working tree except the reserved subtree,
    /// deletions included.
    fn stage_tree(&mut self) -> Result<(), MemoryError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let mut skip_reserved = |path: &std::path::Path, _matched: &[u8]| -> i32 {
            let rendered = path.to_string_lossy().replace('\\', "/");
            i32::from(
                rendered
                    .split('/')
                    .any(|part| part == kernel::RESERVED_PREFIX),
            )
        };
        index
            .add_all(
                ["*"],
                git2::IndexAddOption::DEFAULT,
                Some(&mut skip_reserved),
            )
            .map_err(git_err("stage the tree"))?;
        index
            .update_all(["*"], Some(&mut skip_reserved))
            .map_err(git_err("stage deletions"))?;
        index.write().map_err(git_err("write index"))
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
        let seconds = i64::try_from(plan.t.value().saturating_div(1000)).unwrap_or(0);
        let when = git2::Time::new(seconds, 0);
        let email = plan.of.email();
        let signature = git2::Signature::new(plan.of.actor().as_str(), &email, &when)
            .map_err(git_err("build signature"))?;
        let parents: Vec<git2::Commit> = match self.repo.head() {
            Ok(head) => match head.peel_to_commit() {
                Ok(commit) => vec![commit],
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        };
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::super::provenance::{ModelChoice, Provenance};
    use super::*;
    use kernel::{Address, B3Hash, Effort, RunId};
    use std::path::Path;

    fn resident() -> Provenance {
        Provenance::new(
            RunId::parse("018f5b2a-0000-7000-8000-000000000001").unwrap(),
            Address::parse("work/resident").unwrap(),
            B3Hash::digest(b"a city"),
            ModelChoice {
                id: "test-model".to_owned(),
                effort: Some(Effort::Medium),
            },
        )
    }
    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn a_staged_secret_refuses_the_commit_and_never_echoes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
        write(tmp.path(), "work/leak.env", &format!("KEY={token}"));
        let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
        let err = match checkpoint.wave_pre("work", TimeMs::new(0), &resident()) {
            Err(err) => err,
            Ok(_) => panic!("a staged secret must refuse the commit"),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("work/leak.env"), "{rendered}");
        assert!(
            !rendered.contains(&token) && !rendered.contains("Zx9yQ2mK4pL7"),
            "positions only, never the bytes: {rendered}"
        );
        let ax = err.into_ax();
        assert_eq!(*ax.code(), kernel::AxCode::SecretEgress);
    }

    /// Five trailers, in the order and the spelling a reader outside the
    /// city was promised. Read as git reads them: the message is parsed
    /// line by line off the commit object, which is what
    /// `git interpret-trailers --parse` walks.
    #[test]
    fn every_commit_the_city_makes_names_the_session_that_made_it() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "work/note.md", "a line");
        let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
        let of = resident();
        checkpoint
            .ensure_base("work", TimeMs::new(1_000), &of)
            .unwrap();

        let repo = git2::Repository::open(tmp.path()).unwrap();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        let message = commit.message().unwrap();
        let trailers: Vec<String> = message
            .lines()
            .filter(|line| line.starts_with("Sprawling-"))
            .map(str::to_owned)
            .collect();
        let city = kernel::B3Hash::digest(b"a city").to_string();
        assert_eq!(
            trailers,
            vec![
                format!("Sprawling-Run: {}", of.run()),
                "Sprawling-Actor: work/resident".to_owned(),
                "Sprawling-Model: test-model".to_owned(),
                "Sprawling-Effort: medium".to_owned(),
                format!("Sprawling-City: {city}"),
            ]
        );
        assert!(message.starts_with("checkpoint: work\n\n"), "{message}");

        let author = commit.author();
        assert_eq!(author.name().unwrap(), "work/resident");
        assert_eq!(
            author.email().unwrap(),
            format!("work/resident@{}.sprawling", city.get(..12).unwrap())
        );
    }
}
