// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::GitOid;

use crate::changes::Head;
use crate::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchLine {
    pub number: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withheld {
    pub number: u32,

    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePatch {
    pub lines: Vec<PatchLine>,
    pub withheld: Vec<Withheld>,
}

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> StorageError {
    move |err| StorageError::Checkpoint {
        op,
        detail: err.message().to_owned(),
    }
}

pub fn of_file(
    city_root: &Path,
    base: GitOid,
    head: Head,
    path: &str,
) -> Result<FilePatch, StorageError> {
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
            let hits = kernel::secret::scan(body.as_bytes());
            match hits.first() {
                Some(hit) => withheld.push(Withheld {
                    number,
                    reason: hit.reason().to_owned(),
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

    #[test]
    fn one_file_answers_with_its_patch_and_a_credential_line_is_named_not_echoed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/lex.rs", "one\ntwo\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(1_000), &resident())
                .unwrap(),
        );

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

    #[test]
    fn a_file_that_did_not_move_answers_with_an_empty_patch() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/keep.rs", "unchanged\n");
        let mut checkpoint = Checkpoint::open(root).unwrap();
        let base = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(1_000), &resident())
                .unwrap(),
        );
        write(root, "lab/moved.rs", "new\n");
        let head = oid_of(
            &checkpoint
                .wave_pre(&["lab".to_owned()], TimeMs::new(2_000), &resident())
                .unwrap(),
        );

        let patch = of_file(root, base, Head::Commit(head), "lab/keep.rs").unwrap();
        assert!(patch.lines.is_empty());
        assert!(patch.withheld.is_empty());
    }
}
