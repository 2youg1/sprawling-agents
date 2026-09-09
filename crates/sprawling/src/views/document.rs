// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file of the city, read at the moment of asking and cut to what
//! travels.
//!
//! The same bound `read_building` puts on a building's documents, for
//! the same reason; and one judgement that module never needed, because
//! it only ever read Markdown: a file whose head holds a NUL byte is
//! not text, and showing it as text would show a reader something the
//! file does not say.

use kernel::Address;

use super::holding::Views;
use super::listing::resolve;
use crate::assembly::DOC_BYTES_MAX;

/// How far into a file the text judgement looks. Deep enough that a
/// ledger segment or a Markdown file with one odd byte far down is
/// still text; shallow enough to cost nothing.
const SNIFF_BYTES: usize = 8 * 1024;

impl Views {
    /// The head of one file, or `None` when there is no file to read.
    pub(super) fn document_answer(&self, at: &Address) -> Option<channels::DocumentAnswer> {
        let bytes = std::fs::read(resolve(&self.city_root, Some(at))).ok()?;
        Some(read_document(at.clone(), &bytes))
    }
}

/// The wire shape of one file's bytes. Pure, so the judgement is tested
/// without a disk.
pub(super) fn read_document(at: Address, bytes: &[u8]) -> channels::DocumentAnswer {
    let sniffed = bytes.get(..bytes.len().min(SNIFF_BYTES)).unwrap_or(bytes);
    let binary = sniffed.contains(&0);
    let head = bytes.get(..bytes.len().min(DOC_BYTES_MAX)).unwrap_or(bytes);
    channels::DocumentAnswer {
        at,
        text: if binary {
            String::new()
        } else {
            String::from_utf8_lossy(head).into_owned()
        },
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        truncated: !binary && bytes.len() > DOC_BYTES_MAX,
        binary,
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

    /// The rules that govern a building are readable through the tree,
    /// which is the point of the tree.
    #[test]
    fn a_governing_file_reads_back_whole() {
        let dir = tempfile::tempdir().unwrap();
        crate::assembly::init_city(dir.path()).unwrap();
        let mut views = Views::new(dir.path());
        let rules = Address::parse("hall/.sprawling/BUILDING.md").unwrap();
        let channels::Answer::Document(answer) =
            views.answer(&channels::Query::Document { at: rules.clone() })
        else {
            panic!("Document answers with a document");
        };
        assert_eq!(answer.at, rules);
        assert!(answer.text.contains("confidential"), "{}", answer.text);
        assert!(!answer.truncated);
        assert!(!answer.binary);
        assert_eq!(answer.bytes, u64::try_from(answer.text.len()).unwrap());
    }

    /// A file that is not there is one this view could not look at,
    /// which is what `Unavailable` says.
    #[test]
    fn a_missing_file_is_unavailable_not_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let answer = views.answer(&channels::Query::Document {
            at: Address::parse("hall/nothing.md").unwrap(),
        });
        assert!(
            matches!(answer, channels::Answer::Unavailable { .. }),
            "{answer:?}"
        );
    }

    #[test]
    fn bytes_with_a_nul_are_not_shown_as_text() {
        let at = Address::parse("x/views/db.redb").unwrap();
        let answer = read_document(at, b"redb\0\x01\x02");
        assert!(answer.binary);
        assert_eq!(answer.text, "");
        assert_eq!(answer.bytes, 7);
        assert!(!answer.truncated);
    }

    #[test]
    fn a_long_file_is_cut_and_says_so() {
        let at = Address::parse("lab/Memo.md").unwrap();
        let long = vec![b'a'; DOC_BYTES_MAX.saturating_add(10)];
        let answer = read_document(at, &long);
        assert!(answer.truncated);
        assert_eq!(answer.text.len(), DOC_BYTES_MAX);
        assert_eq!(answer.bytes, u64::try_from(long.len()).unwrap());
    }
}
