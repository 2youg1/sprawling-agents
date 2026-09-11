// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file's patch text between two checkpoints.
//!
//! **This is the separate request `memory::changes` says it is not.**
//! That module answers which files moved and by how much, and its header
//! states why patch text is not its business: patch text is file content
//! on a socket, so it has to be asked for one file at a time and put
//! through the same credential scan a checkpoint puts a staged blob
//! through. This module is that request. The two are not a short and a
//! long version of one answer — one costs what the number of changed
//! files costs, the other costs what one file costs, and merging them
//! would charge every "what changed here" with a whole batch of patches.
//!
//! **The scan is the same scan, not a second copy of it.**
//! `checkpoint::scan_staged` judges a staged blob with
//! `kernel::secret::scan`, and so does this. A line that scan matches is
//! never echoed: the answer carries its line number and the reason, for
//! the reason `scan_staged` gives about its own hits — printing the bytes
//! to prove a leak is the leak.

use std::path::Path;

use kernel::GitOid;

use crate::changes::Head;
use crate::error::MemoryError;

/// One line of patch text, as a reader may see it.
///
/// `number` is the position inside the patch, counted from one, so a
/// withheld line and the lines around it line up in one list without the
/// reader joining two lists by index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchLine {
    pub number: u32,
    pub text: String,
}

/// One line that was not echoed, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withheld {
    pub number: u32,
    /// What matched: a provider's own key shape by name, or the entropy
    /// judgement when no shape claimed it. Never the bytes.
    pub reason: String,
}

/// One file's patch, with what could not be shown named rather than
/// silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePatch {
    pub lines: Vec<PatchLine>,
    pub withheld: Vec<Withheld>,
}

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> MemoryError {
    move |err| MemoryError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

/// The patch text of one file between `base` and `head`.
///
/// One path per call, and the path is required: there is no shape here
/// that answers for a whole batch, because a batch of patches is the
/// thing this module's neighbour refuses to put on a socket.
///
/// A file that did not move between the two commits answers with an
/// empty patch and nothing withheld, which is a different answer from a
/// commit this city never wrote — that one is the caller's to report,
/// because only the caller knows what it asked about.
///
/// # Errors
/// Propagates opening the repository, finding either commit, and walking
/// the difference.
pub fn of_file(
    city_root: &Path,
    base: GitOid,
    head: Head,
    path: &str,
) -> Result<FilePatch, MemoryError> {
    let repo = git2::Repository::open(city_root).map_err(git_err("open the city repository"))?;
    let find = |oid: GitOid| -> Result<git2::Tree<'_>, MemoryError> {
        let parsed =
            git2::Oid::from_str(&oid.to_string()).map_err(git_err("parse a checkpoint"))?;
        repo.find_commit(parsed)
            .map_err(git_err("find a checkpoint"))?
            .tree()
            .map_err(git_err("read a checkpoint tree"))
    };
    let old = find(base)?;
    let mut options = git2::DiffOptions::new();
    options.pathspec(path);
    let diff = match head {
        Head::Commit(oid) => {
            let new = find(oid)?;
            repo.diff_tree_to_tree(Some(&old), Some(&new), Some(&mut options))
        }
        Head::WorkingTree => repo.diff_tree_to_workdir_with_index(Some(&old), Some(&mut options)),
    }
    .map_err(git_err("compare two checkpoints"))?;
    Ok(read(&diff))
}

/// Reads a prepared one-file diff into lines, withholding what the
/// credential scan matched.
///
/// Separate from [`of_file`] because everything above it is git and
/// everything here is this module's own judgement about what a reader
/// may see.
fn read(diff: &git2::Diff<'_>) -> FilePatch {
    let mut lines: Vec<PatchLine> = Vec::new();
    let mut withheld: Vec<Withheld> = Vec::new();
    let mut number: u32 = 0;
    let _walked = diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |_, _, line| {
            number = number.saturating_add(1);
            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content());
            let body = content.trim_end_matches(['\r', '\n']);
            let hits = kernel::scan(body.as_bytes());
            match hits.first() {
                Some(hit) => withheld.push(Withheld {
                    number,
                    reason: hit.provider.unwrap_or("high entropy").to_owned(),
                }),
                None => lines.push(PatchLine {
                    number,
                    text: format!("{origin}{body}"),
                }),
            }
            true
        }),
    );
    FilePatch { lines, withheld }
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

    /// The whole of this card in one case: a person reviewing a change
    /// reads the patch of one file, and the line that carried a key is
    /// reported by number and reason rather than echoed.
    #[test]
    fn one_file_answers_with_its_patch_and_a_credential_line_is_named_not_echoed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\ntwo\n");
        let mut fence = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &fence
                .wave_pre(&["lab".to_owned()], TimeMs::new(1_000), &resident())
                .unwrap(),
        );

        // Against the working tree, because a checkpoint refuses to
        // commit a blob carrying a key at all: the tree on disk is the
        // one place this line can exist, and it is exactly the state a
        // person reviewing a wave in flight is looking at.
        let key = format!("sk-ant-api03-{}", "a1B2c3D4e5".repeat(9));
        write(root, "lab/lex.rs", &format!("one\ntwo\nthree\n{key}\n"));
        write(root, "lab/other.rs", "not asked for\n");

        let patch = of_file(root, base, Head::WorkingTree, "lab/lex.rs").unwrap();
        let shown = patch
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
        assert!(shown.contains("+three"), "the patch text is answered");
        assert!(
            !shown.contains("sk-ant-api03"),
            "a line matching a credential shape is never echoed: {shown}"
        );
        assert!(
            !shown.contains("not asked for"),
            "one file per request, and this was not it"
        );
        assert_eq!(patch.withheld.len(), 1, "one line was held back");
        assert!(
            patch.withheld[0].number > 0,
            "a withheld line says where it was"
        );
        assert!(
            !patch.withheld[0].reason.is_empty(),
            "and why it was held back"
        );
    }

    /// A file nobody touched between the two fences has an empty patch
    /// rather than a failure: "it did not move" is an answer.
    #[test]
    fn a_file_that_did_not_move_answers_with_an_empty_patch() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/keep.rs", "unchanged\n");
        let mut fence = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &fence
                .wave_pre(&["lab".to_owned()], TimeMs::new(1_000), &resident())
                .unwrap(),
        );
        write(root, "lab/moved.rs", "new\n");
        let head = oid_of(
            &fence
                .wave_pre(&["lab".to_owned()], TimeMs::new(2_000), &resident())
                .unwrap(),
        );

        let patch = of_file(root, base, Head::Commit(head), "lab/keep.rs").unwrap();
        assert!(patch.lines.is_empty());
        assert!(patch.withheld.is_empty());
    }
}
