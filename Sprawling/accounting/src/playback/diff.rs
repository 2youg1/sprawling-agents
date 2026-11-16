// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, GitOid};
use storage::Head;

use super::DIFF_MAX_BYTES;
use super::document::{Base, Change, Decimal, DiffLine, FileDiff, HiddenLine};
use super::project::building_of;
use super::reader::Readership;

#[derive(Debug, Clone, Copy)]
pub(super) struct Between {
    pub(super) base: Base,
    pub(super) head: GitOid,
}

pub(super) fn changes(
    city_root: &Path,
    Between { base, head }: Between,
    files: &[String],
    readership: &mut Readership,
) -> Vec<FileDiff> {
    let base = match base {
        Base::Previous(oid) | Base::Parent(oid) => oid,
        Base::Absent => return Vec::new(),
    };
    let mut budget = DIFF_MAX_BYTES;
    files
        .iter()
        .map(|path| FileDiff {
            path: path.clone(),
            change: if closed(path, readership) {
                Change::Withheld
            } else {
                change(city_root, (base, head), path, &mut budget)
            },
        })
        .collect()
}

fn closed(path: &str, readership: &mut Readership) -> bool {
    match Address::parse(path) {
        Ok(addr) => readership.closes(&building_of(&addr)).is_some(),
        Err(_not_an_address) => true,
    }
}

fn change(
    city_root: &Path,
    (base, head): (GitOid, GitOid),
    path: &str,
    budget: &mut usize,
) -> Change {
    let patch = match storage::of_file(city_root, base, Head::Commit(head), path) {
        Ok(patch) => patch,
        Err(_unavailable) => return Change::Missing,
    };
    if !patch.lines.is_empty() || !patch.withheld.is_empty() {
        return shown(patch, budget);
    }
    match binary(city_root, [head, base], path) {
        Ok(true) => Change::Binary,
        Ok(false) => Change::Empty,
        Err(_unreadable) => Change::Missing,
    }
}

fn shown(patch: storage::FilePatch, budget: &mut usize) -> Change {
    let mut lines = Vec::new();
    let mut cut = 0u64;
    for line in patch.lines {
        match budget.checked_sub(line.text.len()) {
            Some(left) if cut == 0 => {
                *budget = left;
                lines.push(DiffLine {
                    number: Decimal(u64::from(line.number)),
                    text: line.text,
                });
            }
            Some(_) | None => {
                *budget = 0;
                cut = cut.saturating_add(1);
            }
        }
    }
    let credential = patch
        .withheld
        .into_iter()
        .map(|held| HiddenLine {
            number: Decimal(u64::from(held.number)),
            reason: held.reason,
        })
        .collect();
    match cut {
        0 => Change::Patch { lines, credential },
        cut => Change::Truncated {
            lines,
            credential,
            cut: Decimal(cut),
        },
    }
}

fn binary(city_root: &Path, commits: [GitOid; 2], path: &str) -> Result<bool, git2::Error> {
    let repo = git2::Repository::open(city_root)?;
    for oid in commits {
        let tree = repo
            .find_commit(git2::Oid::from_str(&oid.to_string())?)?
            .tree()?;
        match tree.get_path(Path::new(path)) {
            Ok(entry) => return Ok(repo.find_blob(entry.id())?.is_binary()),
            Err(err) if err.code() == git2::ErrorCode::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(false)
}
