// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint fences: base, wave pre/post.

use std::path::Path;

use kernel::{Payload, TimeMs};
use serde_json::{Map, Value};

use crate::error::MemoryError;

pub(crate) const IDENTITY_NAME: &str = "sprawling";
pub(crate) const IDENTITY_EMAIL: &str = "sprawling@local";

pub struct Checkpoint {
    pub(crate) repo: git2::Repository,
}

pub(crate) fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> MemoryError {
    move |err| MemoryError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

impl Checkpoint {
    /// Opens the city repository, initialising one when absent. The
    /// genesis commit is the first wave's, not this call's: an empty
    /// repository is a valid state, and inventing history here would
    /// make the first checkpoint unattributable.
    pub fn open(city_root: &Path) -> Result<Checkpoint, MemoryError> {
        let repo = match git2::Repository::open(city_root) {
            Ok(repo) => repo,
            Err(_) => git2::Repository::init(city_root).map_err(git_err("init repository"))?,
        };
        // The city's files round-trip byte for byte, whatever this
        // machine's git is configured to do to other people's
        // repositories. A checkout that rewrote line endings would make
        // a file disagree with the hash the ledger holds for it, and the
        // disagreement would look like corruption rather than like a
        // setting. Set on every open, because the setting is a property
        // of this repository rather than of the moment it was created.
        repo.config()
            .and_then(|mut config| config.set_bool("core.autocrlf", false))
            .map_err(git_err("pin the repository's line endings"))?;
        Ok(Checkpoint { repo })
    }

    /// Makes sure the city has one commit, and makes no more than that.
    ///
    /// A worktree branches from a commit, so a city that has never been
    /// fenced cannot lend a tree. Committing on every dispatch would
    /// instead move the trunk under every request already waiting, and a
    /// fast-forward merge would then refuse work nobody had touched.
    /// Returns the commit it made, or `None` when there already was one.
    ///
    /// # Errors
    /// Propagates whatever staging and committing report.
    pub fn ensure_base(
        &mut self,
        scope: &str,
        t: TimeMs,
        who: &str,
    ) -> Result<Option<Payload>, MemoryError> {
        if self.repo.head().is_ok() {
            return Ok(None);
        }
        self.wave_pre(scope, t, who).map(Some)
    }

    /// The pre-wave fence: stage everything under `scope`, scan it, and
    /// commit at the injected time. Returns the `checkpoint_committed`
    /// payload.
    pub fn wave_pre(&mut self, scope: &str, t: TimeMs, who: &str) -> Result<Payload, MemoryError> {
        let files = self.stage_scope(scope)?;
        self.scan_staged()?;
        let oid = self.commit(t, who, &format!("checkpoint: {scope}"))?;
        let mut map = Map::new();
        map.insert("oid".to_owned(), Value::String(oid));
        map.insert("scope".to_owned(), Value::String(scope.to_owned()));
        map.insert(
            "files".to_owned(),
            Value::Array(files.into_iter().map(Value::String).collect()),
        );
        Payload::new(map).map_err(|source| MemoryError::Draft { source })
    }

    /// The post-wave sweep: every path present at `pre_oid` and gone
    /// from the working tree becomes a `file_discarded` payload whose
    /// restoration points back into that commit.
    pub fn wave_post(&mut self, pre_oid: &str) -> Result<Vec<Payload>, MemoryError> {
        let oid = git2::Oid::from_str(pre_oid).map_err(git_err("parse checkpoint oid"))?;
        let commit = self
            .repo
            .find_commit(oid)
            .map_err(git_err("find checkpoint commit"))?;
        let tree = commit.tree().map_err(git_err("read checkpoint tree"))?;
        // git already knows what is missing, so it is asked once instead
        // of being told the answer file by file. Walking the whole tree
        // and calling `exists` on every blob cost one `format!`, one
        // `PathBuf` and one filesystem stat per tracked file, after every
        // wave, whether or not that wave touched anything.
        let mut options = git2::DiffOptions::new();
        options.include_typechange(true);
        let diff = self
            .repo
            .diff_tree_to_workdir(Some(&tree), Some(&mut options))
            .map_err(git_err("diff checkpoint against the work tree"))?;
        let mut deleted: Vec<String> = Vec::new();
        for delta in diff.deltas() {
            if delta.status() != git2::Delta::Deleted {
                continue;
            }
            if let Some(path) = delta.old_file().path().and_then(|p| p.to_str()) {
                deleted.push(path.replace('\\', "/"));
            }
        }
        // Sorted here rather than trusted from the diff: the order these
        // records land in is part of what a replay reproduces.
        deleted.sort();
        let mut payloads = Vec::new();
        for path in deleted {
            let mut map = Map::new();
            map.insert(
                "paths".to_owned(),
                Value::Array(vec![Value::String(format!("file:{path}"))]),
            );
            let mut restoration = Map::new();
            restoration.insert(
                "tracked".to_owned(),
                Value::String(format!("file:{path}@{pre_oid}")),
            );
            map.insert("restoration".to_owned(), Value::Object(restoration));
            payloads.push(Payload::new(map).map_err(|source| MemoryError::Draft { source })?);
        }
        Ok(payloads)
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
    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }
    fn oid_of(payload: &Payload) -> String {
        serde_json::to_value(payload).unwrap()["oid"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn a14_the_fence_precedes_the_deletion_it_restores() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "work/keep.txt", "kept");
        write(tmp.path(), "work/doomed.txt", "about to go");
        let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();

        let pre = checkpoint
            .wave_pre("work", TimeMs::new(1_700_000_000_000), "resident")
            .unwrap();
        let pre_oid = oid_of(&pre);
        let files = serde_json::to_value(&pre).unwrap();
        assert_eq!(files["files"].as_array().unwrap().len(), 2);

        // The wave deletes a file.
        std::fs::remove_file(tmp.path().join("work/doomed.txt")).unwrap();

        let discards = checkpoint.wave_post(&pre_oid).unwrap();
        assert_eq!(discards.len(), 1);
        let value = serde_json::to_value(&discards[0]).unwrap();
        assert_eq!(value["paths"][0], "file:work/doomed.txt");
        assert_eq!(
            value["restoration"]["tracked"],
            format!("file:work/doomed.txt@{pre_oid}"),
            "the restoration names a commit that already exists"
        );
    }

    #[test]
    fn the_scope_is_the_boundary_and_outside_it_nothing_is_staged() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "work/mine.txt", "in domain");
        write(tmp.path(), "elsewhere/theirs.txt", "not mine");
        let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
        let pre = checkpoint
            .wave_pre("work", TimeMs::new(1_700_000_000_000), "resident")
            .unwrap();
        let files = serde_json::to_value(&pre).unwrap();
        let staged: Vec<String> = files["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(staged, vec!["work/mine.txt".to_owned()]);
    }

    #[test]
    fn a_wave_pays_for_what_it_changed_rather_than_for_the_whole_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
        write(tmp.path(), "work/smuggled.env", &format!("KEY={token}"));
        let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
        checkpoint.stage_scope("work").unwrap();
        checkpoint
            .commit(TimeMs::new(1_000), "resident", "past the fence")
            .unwrap();

        // An ordinary wave that touches a different file. What it pays
        // for is its own change; the blob it did not touch is not read.
        write(tmp.path(), "work/ordinary.txt", "nothing to see");
        checkpoint
            .wave_pre("work", TimeMs::new(2_000), "resident")
            .expect("an unchanged blob is not re-examined");

        // And the guard still bites on this wave's own writing, which is
        // the half of the property the narrowing must not cost.
        write(tmp.path(), "work/fresh.env", &format!("KEY={token}"));
        let err = match checkpoint.wave_pre("work", TimeMs::new(3_000), "resident") {
            Err(err) => err,
            Ok(_) => panic!("a secret arriving with this wave must refuse the commit"),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("work/fresh.env"), "{rendered}");
        assert!(
            !rendered.contains("work/smuggled.env"),
            "only what this wave staged is reported: {rendered}"
        );
    }

    #[test]
    fn an_unchanged_wave_still_commits_so_the_chain_rebuilds() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "work/steady.txt", "unchanged");
        let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
        let first = oid_of(
            &checkpoint
                .wave_pre("work", TimeMs::new(1_000), "resident")
                .unwrap(),
        );
        let second = oid_of(
            &checkpoint
                .wave_pre("work", TimeMs::new(2_000), "resident")
                .unwrap(),
        );
        assert_ne!(first, second, "each fence is its own commit");
        assert!(checkpoint.wave_post(&second).unwrap().is_empty());
    }

    #[test]
    fn the_same_script_at_the_same_time_produces_the_same_commit() {
        let build = |dir: &Path| -> String {
            write(dir, "work/a.txt", "alpha");
            let mut checkpoint = Checkpoint::open(dir).unwrap();
            oid_of(
                &checkpoint
                    .wave_pre("work", TimeMs::new(1_700_000_000_000), "resident")
                    .unwrap(),
            )
        };
        let one = tempfile::tempdir().unwrap();
        let two = tempfile::tempdir().unwrap();
        assert_eq!(
            build(one.path()),
            build(two.path()),
            "time is a parameter, so the oid is reproducible"
        );
    }
}
