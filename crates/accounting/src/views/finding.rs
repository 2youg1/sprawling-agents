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

use std::path::Path;

use kernel::Address;

/// The files under `under` whose name holds `text`, without regard to
/// case.
///
/// Takes the city root rather than the views: it reads the disk, and
/// runs after the view lock is released, as `listing_answer` does.
pub(super) fn find_answer(_city_root: &Path, under: Address, text: String) -> wire::FindAnswer {
    wire::FindAnswer {
        under,
        text,
        paths: Vec::new(),
        walked: wire::Walked::Whole,
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
