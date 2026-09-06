// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint scans: staged secrets and scoped commits.

use kernel::{TimeMs, scan};

use crate::error::MemoryError;

use super::fence::{Checkpoint, IDENTITY_EMAIL, IDENTITY_NAME, git_err};

impl Checkpoint {
    /// Scans what this checkpoint would newly write into the tree.
    /// Reports how many shapes matched and where, never what matched.
    pub fn scan_staged(&mut self) -> Result<(), MemoryError> {
        let index = self.repo.index().map_err(git_err("read index"))?;
        let mut hits: Vec<String> = Vec::new();
        match self.head_tree()? {
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
    fn head_tree(&self) -> Result<Option<git2::Tree<'_>>, MemoryError> {
        let Ok(head) = self.repo.head() else {
            return Ok(None);
        };
        let commit = head
            .peel_to_commit()
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

    /// Commits the index at the injected time. An unchanged tree still
    /// commits: a rebuildable chain is worth more than a saved object.
    pub(crate) fn commit(
        &mut self,
        t: TimeMs,
        who: &str,
        message: &str,
    ) -> Result<String, MemoryError> {
        let mut index = self.repo.index().map_err(git_err("read index"))?;
        let tree_oid = index.write_tree().map_err(git_err("write tree"))?;
        let tree = self
            .repo
            .find_tree(tree_oid)
            .map_err(git_err("find staged tree"))?;
        let seconds = i64::try_from(t.value().saturating_div(1000)).unwrap_or(0);
        let when = git2::Time::new(seconds, 0);
        let signature = git2::Signature::new(IDENTITY_NAME, IDENTITY_EMAIL, &when)
            .map_err(git_err("build signature"))?;
        let parents: Vec<git2::Commit> = match self.repo.head() {
            Ok(head) => match head.peel_to_commit() {
                Ok(commit) => vec![commit],
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        let full_message = format!("{message}\n\nactor: {who}\n");
        let oid = self
            .repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                &full_message,
                &tree,
                &parent_refs,
            )
            .map_err(git_err("commit checkpoint"))?;
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
    use super::*;
    use std::path::Path;
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
        let err = match checkpoint.wave_pre("work", TimeMs::new(0), "resident") {
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
}
