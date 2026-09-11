// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Checkpoint fences: base, wave pre/post.

use std::path::Path;

use kernel::{Payload, TimeMs};
use serde_json::{Map, Value};

use crate::error::MemoryError;

use super::provenance::Provenance;
use super::scan::CommitPlan;

/// How a fence names itself in a commit subject. The whole set, because
/// a fence that showed one of several prefixes would read like a fence
/// that staged one of them.
fn subject_of(scopes: &[String]) -> String {
    if scopes.is_empty() {
        return "checkpoint: .".to_owned();
    }
    format!("checkpoint: {}", scopes.join(" "))
}

/// Where a wave fence is filed: under `refs/sprawling/`, which no
/// branch listing, push or `git log` walks by accident.
fn fence_ref(of: &Provenance, seq: u64) -> String {
    format!("refs/sprawling/runs/{}/{seq}", of.run())
}

/// The `checkpoint_committed` payload, in one place so a fence and a
/// base commit cannot describe themselves differently.
///
/// It carries what the commit's own trailers carry that the record
/// cannot say for itself: the model and the effort. The run and the
/// actor are the record's identity and are not repeated here.
fn committed(
    oid: git2::Oid,
    of: &Provenance,
    scopes: &[String],
    files: Vec<String>,
) -> Result<Payload, MemoryError> {
    let mut map = of.model_fields();
    map.insert("oid".to_owned(), Value::String(oid.to_string()));
    map.insert(
        "scope".to_owned(),
        Value::Array(scopes.iter().cloned().map(Value::String).collect()),
    );
    map.insert(
        "files".to_owned(),
        Value::Array(files.into_iter().map(Value::String).collect()),
    );
    Payload::new(map).map_err(|source| MemoryError::Draft { source })
}

pub struct Checkpoint {
    pub(crate) repo: git2::Repository,
    /// The last commit this handle made, fence or landing. The scan
    /// compares against it, because a fence no longer moves HEAD.
    pub(crate) last: Option<git2::Oid>,
    /// How many fences this handle has raised. The name only has to be
    /// unique among them: the Ledger's `oid` answers every question
    /// about a wave, and the reference only keeps the commit reachable.
    fences: u64,
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
        Ok(Checkpoint {
            repo,
            last: None,
            fences: 0,
        })
    }

    /// Makes sure the city has one commit, and makes no more than that.
    ///
    /// A worktree branches from a commit, so a city that has never been
    /// fenced cannot lend a tree. Committing on every dispatch would
    /// move the trunk under every request already waiting. Returns the
    /// commit it made, or `None` when there already was one.
    ///
    /// # Errors
    /// Propagates whatever staging and committing report.
    pub fn ensure_base(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Option<Payload>, MemoryError> {
        if self.repo.head().is_ok() {
            return Ok(None);
        }
        let files = self.stage_scopes(scopes)?;
        self.scan_staged()?;
        // The one fence that moves the branch: a worktree branches from a
        // commit, and a city that has none can lend no tree.
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            onto_head: true,
        })?;
        committed(oid, of, scopes, files).map(Some)
    }

    /// The pre-wave fence: stage everything under `scopes`, scan it, and
    /// commit at the injected time. Returns the `checkpoint_committed`
    /// payload.
    ///
    /// **`scopes` is the run's write domain, not its room** (memory-SPEC
    /// section 8-18). The two were allowed to differ once, and every
    /// file a resident wrote between them - a building's own documents,
    /// a second declared prefix - was staged by no fence, reported by no
    /// `changes` query, and restorable from no `file_discarded` record.
    ///
    /// **The branch does not move**: the commit is written
    /// with no reference update and pointed at by
    /// `refs/sprawling/runs/<run>/<seq>`, so a person whose own folder
    /// became this city keeps their own history instead of one
    /// `checkpoint:` line per tool wave. The oid means what it always
    /// meant - the tree is there and `wave_post` restores from it.
    ///
    /// # Errors
    /// Propagates staging, the staged-secret scan, the commit, and a
    /// reference the repository will not write.
    pub fn wave_pre(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
    ) -> Result<Payload, MemoryError> {
        let files = self.stage_scopes(scopes)?;
        self.scan_staged()?;
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            onto_head: false,
        })?;
        let seq = self.fences;
        self.fences = self.fences.saturating_add(1);
        self.repo
            .reference(&fence_ref(of, seq), oid, true, "sprawling: a wave fence")
            .map_err(git_err("file a wave fence"))?;
        committed(oid, of, scopes, files)
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
mod tests;
