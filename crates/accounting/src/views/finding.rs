// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The files under one address whose name holds a text, found by one
//! bounded walk of the tree `Query::Listing` reads
//! (`crates/wire/spec/Answer/Find.lean` §8-82).
//!
//! The disk is the authority, as it is for `listing`: an index kept
//! beside the tree would be a second copy of what the tree says (wire
//! D17). The walk goes level by level, each level in name order, so the
//! shallow files come first and two machines answer in one order.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use kernel::Address;

use super::holding::Views;
use super::prepared::Prepared;

impl Views {
    /// The walk reads the disk, so it runs after the views are released.
    pub(in crate::views) fn find_ask(&self, under: &Address, text: &str) -> Prepared {
        Prepared::Find(self.city_root.clone(), under.clone(), text.to_owned())
    }
}

/// The files under `under` whose name holds `text`, without regard to
/// case.
///
/// Takes the city root rather than the views: it reads the disk, and
/// runs after the view lock is released, as `listing_answer` does.
pub(in crate::views) fn find_answer(
    city_root: &Path,
    under: Address,
    text: String,
) -> wire::FindAnswer {
    let (paths, walked) = walk(&city_root.join(under.as_str()), &text.to_lowercase());
    wire::FindAnswer {
        under,
        text,
        paths,
        walked,
    }
}

/// One level at a time from `root`, each level in name order, until the
/// tree ends or a bound is reached.
fn walk(root: &Path, needle: &str) -> (Vec<Address>, wire::Walked) {
    let mut found = Vec::new();
    let mut seen: usize = 0;
    let mut levels: VecDeque<(PathBuf, String)> =
        VecDeque::from([(root.to_path_buf(), String::new())]);
    while let Some((dir, prefix)) = levels.pop_front() {
        for (name, is_dir) in entries(&dir) {
            seen = seen.saturating_add(1);
            if found.len() >= wire::FIND_MAX || seen > wire::FIND_WALK_MAX {
                return (found, wire::Walked::Cut);
            }
            let relative = format!("{prefix}{name}");
            if is_dir {
                if name != kernel::GIT_METADATA {
                    levels.push_back((dir.join(&name), format!("{relative}/")));
                }
            } else if name.to_lowercase().contains(needle)
                && let Ok(path) = Address::parse(&relative)
            {
                found.push(path);
            }
        }
    }
    (found, wire::Walked::Whole)
}

/// One directory's names in name order, each with whether it is a
/// directory to descend into. A link is neither descended into nor
/// listed, because a link to a parent turns the walk into a circle; a
/// directory that cannot be read has no names, as `listing` says.
fn entries(dir: &Path) -> Vec<(String, bool)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<(String, bool)> = read
        .flatten()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            (!kind.is_symlink()).then(|| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    kind.is_dir(),
                )
            })
        })
        .collect();
    names.sort_unstable();
    names
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

    fn address(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    fn write(root: &Path, relative: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    /// A name matches without regard to case, a directory's own name
    /// never does, the repository's metadata is not walked, and the
    /// shallow file comes before the deep one.
    #[test]
    fn the_files_whose_name_holds_the_text_come_shallow_first() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "lab/notes/deep/Plan-B.md");
        write(root, "lab/plan.md");
        write(root, "lab/planning/readme.md");
        write(root, "lab/.git/plan-object");
        write(root, "lab/other.txt");
        write(root, "elsewhere/plan.md");

        let answer = find_answer(root, address("lab"), "PLAN".to_owned());

        assert_eq!(
            answer,
            wire::FindAnswer {
                under: address("lab"),
                text: "PLAN".to_owned(),
                paths: vec![address("plan.md"), address("notes/deep/Plan-B.md")],
                walked: wire::Walked::Whole,
            }
        );
    }

    /// More matches than one answer carries: the answer stops at the
    /// bound and says the walk was cut.
    #[test]
    fn more_matches_than_the_bound_are_cut_and_say_so() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let over = wire::FIND_MAX.checked_add(3).unwrap();
        for at in 0..over {
            write(root, &format!("lab/note-{at:03}.md"));
        }

        let answer = find_answer(root, address("lab"), "note".to_owned());

        assert_eq!(answer.paths.len(), wire::FIND_MAX);
        assert_eq!(answer.paths[0], address("note-000.md"));
        assert_eq!(answer.walked, wire::Walked::Cut);
    }

    /// An address with nothing under it is an empty answer, not a
    /// refusal: the page asked about a tree, and that tree is empty.
    #[test]
    fn a_missing_address_finds_nothing_and_walked_it_whole() {
        let dir = tempfile::tempdir().unwrap();
        let answer = find_answer(dir.path(), address("nowhere"), String::new());
        assert_eq!(answer.paths, Vec::<Address>::new());
        assert_eq!(answer.walked, wire::Walked::Whole);
    }
}
