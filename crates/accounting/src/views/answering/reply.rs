// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A model's reply, sent back by the page that holds its text, laid out
//! by the grammar a preview is (accounting-SPEC.md 8-29, wire-SPEC.md
//! 8-75).
//!
//! Reads nothing of the city: the answer depends on the text in the
//! question alone. Every judgement - where a streaming reply's closure
//! point is, where a window ends, what the blocks are - is
//! `documents::reply`'s.

use documents::ReplyState;

use super::super::prepared::unavailable;

/// The blocks of `text` that can no longer change, or `Unavailable`
/// when the text is not one the grammar reads (it holds a NUL).
pub(in crate::views) fn reply_answer(text: &str, state: ReplyState) -> wire::Answer {
    match documents::reply(text, state) {
        Ok(laid) => wire::Answer::Reply(Box::new(laid)),
        // The page draws text the grammar does not read as it arrived,
        // which is what it does with the open tail anyway.
        Err(_) => unavailable("Reply".to_owned()),
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
    use std::path::Path;

    use documents::{Block, Inline, Laid, Preview, ReplyState, Span};
    use kernel::B3Hash;
    use storage::Cas;

    use crate::views::Views;

    /// A reply carrying a heading, a list, a table, a fenced block and a
    /// footnote.
    const SAID: &str = "# Plan\n\nTwo steps, **in order**:\n\n- read `lib.rs`\n- write the test\n\n| step | cost |\n|---|---|\n| read | 1 |\n\n```rust\nlet a = 1;\n```\n\nDone.[^1]\n\n[^1]: Checked by hand.\n";

    fn kept(dir: &Path, bytes: &[u8]) -> B3Hash {
        Cas::open(&kernel::layout::CityLayout::new(dir).cas())
            .unwrap()
            .put(bytes)
            .unwrap()
    }

    fn ask_reply(views: &mut Views, text: &str, state: ReplyState) -> wire::Answer {
        views.answer(&wire::Query::Reply {
            text: text.to_owned(),
            state,
        })
    }

    /// The finish line of U9's Rust half: one reply's text, read through
    /// the conversation's door and through the document preview's, is one
    /// tree.
    #[test]
    fn a_settled_reply_reads_as_the_preview_of_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let version = kept(dir.path(), SAID.as_bytes());
        let size = u64::try_from(SAID.len()).unwrap();
        let mut views = Views::new(dir.path());
        let wire::Answer::Preview(previewed) = views.answer(&wire::Query::Preview {
            version,
            viewport: Span::new(0, size).unwrap(),
        }) else {
            panic!("the version was kept");
        };
        let Preview::Laid(laid) = previewed.preview else {
            panic!("UTF-8 is laid out");
        };
        assert!(!laid.blocks.is_empty());
        assert_eq!(
            ask_reply(&mut views, SAID, ReplyState::Settled),
            wire::Answer::Reply(Box::new(laid))
        );
    }

    /// A reply cut inside its second paragraph answers the heading and
    /// the paragraph a blank line closed, and stops where the open one
    /// begins.
    #[test]
    fn a_streaming_reply_answers_only_its_closed_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let said = "# Heading\n\nFirst paragraph.\n\nSecond, still";
        assert_eq!(
            ask_reply(&mut views, said, ReplyState::Streaming),
            wire::Answer::Reply(Box::new(Laid {
                span: Span::new(0, 29).unwrap(),
                blocks: vec![
                    Block::Heading {
                        span: Span::new(0, 9).unwrap(),
                        level: 1,
                        inline: vec![Inline::Text("Heading".to_owned())],
                    },
                    Block::Paragraph {
                        span: Span::new(11, 27).unwrap(),
                        inline: vec![Inline::Text("First paragraph.".to_owned())],
                    },
                ],
            }))
        );
    }

    #[test]
    fn a_reply_holding_a_nul_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        assert_eq!(
            ask_reply(&mut views, "a\0b", ReplyState::Settled),
            wire::Answer::Unavailable {
                query: "Reply".to_owned(),
            }
        );
    }
}
