// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A preview's own judgements: an empty window, a block with nothing in
//! it and an encoding the grammar does not read are three shapes (A4),
//! and a version longer than a window is read window by window, each
//! ending on a block (D25). The corpus is `lowering::tests`.

use super::*;

fn laid(source: &str) -> Vec<Block> {
    let bytes = source.as_bytes();
    let size = u64::try_from(bytes.len()).unwrap();
    match preview(
        Encoding::Utf8,
        Lifted { at: 0, bytes, size },
        Span::new(0, size).unwrap(),
    )
    .unwrap()
    {
        Preview::Laid(Laid { span, blocks }) => {
            assert_eq!(
                span,
                Span::new(0, size).unwrap(),
                "a whole version is read whole"
            );
            blocks
        }
        Preview::Unsupported { encoding } => panic!("{encoding:?} was not laid out"),
    }
}

/// The bytes the first `needle` in `source` occupies.
fn at(source: &str, needle: &str) -> Span {
    let start = source.find(needle).unwrap();
    let end = start + needle.len();
    Span::new(u64::try_from(start).unwrap(), u64::try_from(end).unwrap()).unwrap()
}

fn text(words: &str) -> Inline {
    Inline::Text(words.to_owned())
}

/// A4: nothing here, something not drawn, and a version not read as
/// Markdown are three different answers.
#[test]
fn an_empty_window_an_empty_block_and_an_unread_encoding_are_three_shapes() {
    let empty = preview(
        Encoding::Utf8,
        Lifted {
            at: 0,
            bytes: b"",
            size: 0,
        },
        Span::at(0),
    )
    .unwrap();
    assert_eq!(
        empty,
        Preview::Laid(Laid {
            span: Span::at(0),
            blocks: Vec::new(),
        })
    );
    let fence = "```\n```\n";
    assert_eq!(
        laid(fence),
        vec![Block::Code {
            span: at(fence, "```\n```"),
            info: String::new(),
            text: String::new(),
        }]
    );
    let wide: Vec<u8> = "\u{feff}# Title"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let size = u64::try_from(wide.len()).unwrap();
    assert_eq!(
        preview(
            Encoding::Utf16Le,
            Lifted {
                at: 0,
                bytes: &wide,
                size,
            },
            Span::new(0, size).unwrap(),
        )
        .unwrap(),
        Preview::Unsupported {
            encoding: Encoding::Utf16Le,
        }
    );
}

/// A version three windows long, read on from each answer's end: every
/// window but the last ends where a paragraph ends, and every paragraph
/// is laid out exactly once.
#[test]
fn reading_on_from_each_preview_lays_out_every_paragraph_once() {
    let paragraphs: Vec<String> = (0..2_500)
        .map(|index| format!("Paragraph {index} {}", "x".repeat(60)))
        .collect();
    let source: String = paragraphs
        .iter()
        .map(|line| format!("{line}\n\n"))
        .collect();
    let bytes = source.as_bytes();
    let size = u64::try_from(bytes.len()).unwrap();
    assert!(size > 2 * crate::WINDOW_BYTES_MAX);
    let mut read = Vec::new();
    let mut from = 0;
    while from < size {
        let Preview::Laid(Laid { span, blocks }) = preview(
            Encoding::Utf8,
            Lifted { at: 0, bytes, size },
            Span::new(from, size).unwrap(),
        )
        .unwrap() else {
            panic!("UTF-8 is laid out");
        };
        assert_eq!(span.start(), from);
        assert!(span.end() > from, "every window moves the reader on");
        if span.end() < size {
            let last = blocks.last().map(|block| {
                let Block::Paragraph { span, .. } = block else {
                    panic!("{block:?}");
                };
                span.end()
            });
            assert_eq!(
                last,
                Some(span.end()),
                "a window ends where a paragraph ends"
            );
        }
        for block in blocks {
            let Block::Paragraph { inline, .. } = block else {
                panic!("{block:?}");
            };
            let [Inline::Text(words)] = inline.as_slice() else {
                panic!("{inline:?}");
            };
            read.push(words.clone());
        }
        from = span.end();
    }
    assert_eq!(read, paragraphs);
}

/// A paragraph longer than a window: the window stays as it was cut,
/// because there is no earlier block end to stop at.
#[test]
fn a_window_whose_only_block_runs_on_stays_as_cut() {
    let source = "x".repeat(70_000);
    let bytes = source.as_bytes();
    let size = u64::try_from(bytes.len()).unwrap();
    let max = crate::WINDOW_BYTES_MAX;
    let window = "x".repeat(usize::try_from(max).unwrap());
    assert_eq!(
        preview(
            Encoding::Utf8,
            Lifted { at: 0, bytes, size },
            Span::new(0, size).unwrap(),
        )
        .unwrap(),
        Preview::Laid(Laid {
            span: Span::new(0, max).unwrap(),
            blocks: vec![Block::Paragraph {
                span: Span::new(0, max).unwrap(),
                inline: vec![text(&window)],
            }],
        })
    );
}

/// A reply carrying the constructs a model writes most: a heading, a
/// list, a table, a fenced block and a closing paragraph.
const REPLY: &str = "# Plan\n\nTwo steps, **in order**:\n\n- read `lib.rs`\n- write the test\n\n| step | cost |\n|---|---|\n| read | 1 |\n\n```rust\nlet a = 1;\n```\n\nDone.\n";

fn version_of(source: &str) -> Laid {
    let bytes = source.as_bytes();
    let size = u64::try_from(bytes.len()).unwrap();
    match preview(
        Encoding::Utf8,
        Lifted { at: 0, bytes, size },
        Span::new(0, size).unwrap(),
    )
    .unwrap()
    {
        Preview::Laid(laid) => laid,
        Preview::Unsupported { encoding } => panic!("{encoding:?} was not laid out"),
    }
}

/// D30: a settled reply is laid out as a version holding exactly its
/// text would be previewed, footnotes and all.
#[test]
fn a_settled_reply_is_laid_out_as_a_preview_of_its_text() {
    let said = format!("{REPLY}\nSee the note.[^1]\n\n[^1]: The note.\n");
    let laid = reply(&said, ReplyState::Settled).unwrap();
    assert_eq!(laid, version_of(&said));
    assert_eq!(laid.blocks.len(), 8, "{laid:?}");
}

/// D31: written out one byte at a time, a streaming reply lays out a
/// prefix of the blocks its settled text does, ends where its open tail
/// begins, and reaches the whole tree once a blank line closes the last
/// block.
#[test]
fn a_streaming_reply_lays_out_only_what_can_no_longer_change() {
    let whole = reply(REPLY, ReplyState::Settled).unwrap();
    let mut grown = 0;
    for end in (0..=REPLY.len()).filter(|end| REPLY.is_char_boundary(*end)) {
        let laid = reply(REPLY.get(..end).unwrap(), ReplyState::Streaming).unwrap();
        assert_eq!(laid.span.start(), 0);
        assert!(laid.span.end() <= u64::try_from(end).unwrap());
        assert_eq!(
            laid.blocks,
            whole.blocks[..laid.blocks.len()].to_vec(),
            "{end}"
        );
        grown = grown.max(laid.blocks.len());
    }
    assert_eq!(
        grown + 1,
        whole.blocks.len(),
        "the last block waits for its blank line"
    );
    let closed = reply(&format!("{REPLY}\n"), ReplyState::Streaming).unwrap();
    assert_eq!(closed.blocks, whole.blocks);
    let open = reply("# Plan\n\nTwo steps", ReplyState::Streaming).unwrap();
    assert_eq!(
        open,
        Laid {
            span: Span::new(0, 8).unwrap(),
            blocks: vec![Block::Heading {
                span: Span::new(0, 6).unwrap(),
                level: 1,
                inline: vec![text("Plan")],
            }],
        }
    );
}
