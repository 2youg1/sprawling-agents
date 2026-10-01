// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The grammar's corpus. The CommonMark cases are the ones
//! `client/src/core/prose.test.ts` pinned for the page's own reader; the
//! rest are the extensions this grammar turns on and the three
//! judgements it makes around comrak's tree (D22-D25).

use super::target::admitted;
use super::*;

/// The blocks of a whole version read in `encoding`.
fn laid_in(encoding: Encoding, source: &str) -> Vec<Block> {
    let bytes = source.as_bytes();
    let size = u64::try_from(bytes.len()).unwrap();
    match preview(
        encoding,
        Lifted { at: 0, bytes, size },
        Span::new(0, size).unwrap(),
    )
    .unwrap()
    {
        Preview::Laid { span, blocks } => {
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

fn laid(source: &str) -> Vec<Block> {
    laid_in(Encoding::Utf8, source)
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

fn paragraph(source: &str, needle: &str, inline: Vec<Inline>) -> Block {
    Block::Paragraph {
        span: at(source, needle),
        inline,
    }
}

fn item(source: &str, needle: &str, check: Check, blocks: Vec<Block>) -> ListItem {
    ListItem {
        span: at(source, needle),
        check,
        blocks,
    }
}

/// prose.test.ts: a list, a table and a code block, each closed, then a
/// paragraph still being said.
#[test]
fn a_list_a_table_and_a_code_block_read_as_their_own_blocks() {
    let source = "- `crates/kernel`\n- **Ledger**\n\n| 步骤 | 做什么 |\n|---|---|\n| 1 | 读 |\n\n```rust\nfn main() {}\n\n```\nThen the next par";
    assert_eq!(
        laid(source),
        vec![
            Block::List {
                span: at(source, "- `crates/kernel`\n- **Ledger**"),
                order: Order::Bullet,
                spacing: Spacing::Tight,
                items: vec![
                    item(
                        source,
                        "- `crates/kernel`",
                        Check::NotATask,
                        vec![paragraph(
                            source,
                            "`crates/kernel`",
                            vec![Inline::Code("crates/kernel".to_owned())]
                        )]
                    ),
                    item(
                        source,
                        "- **Ledger**",
                        Check::NotATask,
                        vec![paragraph(
                            source,
                            "**Ledger**",
                            vec![Inline::Strong(vec![text("Ledger")])]
                        )]
                    ),
                ],
            },
            Block::Table {
                span: at(source, "| 步骤 | 做什么 |\n|---|---|\n| 1 | 读 |"),
                align: vec![Align::None, Align::None],
                head: Row {
                    cells: vec![vec![text("步骤")], vec![text("做什么")]],
                },
                body: vec![Row {
                    cells: vec![vec![text("1")], vec![text("读")]],
                }],
            },
            Block::Code {
                span: at(source, "```rust\nfn main() {}\n\n```"),
                info: "rust".to_owned(),
                text: "fn main() {}\n\n".to_owned(),
            },
            paragraph(source, "Then the next par", vec![text("Then the next par")]),
        ]
    );
}

/// prose.test.ts: a fence still open keeps its blank lines.
#[test]
fn an_open_fence_keeps_its_blank_lines() {
    let source = "Before.\n\n```rust\nfn a() {}\n\nfn b";
    assert_eq!(
        laid(source),
        vec![
            paragraph(source, "Before.", vec![text("Before.")]),
            Block::Code {
                span: at(source, "```rust\nfn a() {}\n\nfn b"),
                info: "rust".to_owned(),
                text: "fn a() {}\n\nfn b".to_owned(),
            },
        ]
    );
}

/// prose.test.ts: a blank line ends a paragraph, a line break does not;
/// a heading is one line.
#[test]
fn a_paragraph_ends_at_a_blank_line_and_a_heading_at_its_line() {
    let joined = "one line\nsame paragraph";
    assert_eq!(
        laid(joined),
        vec![paragraph(
            joined,
            joined,
            vec![text("one line"), Inline::SoftBreak, text("same paragraph")]
        )]
    );
    let titled = "# Title\nbody";
    assert_eq!(
        laid(titled),
        vec![
            Block::Heading {
                span: at(titled, "# Title"),
                level: 1,
                inline: vec![text("Title")],
            },
            paragraph(titled, "body", vec![text("body")]),
        ]
    );
}

/// A byte-order mark and every kind of line ending leave each block on
/// its own bytes of the version.
#[test]
fn block_spans_are_the_version_bytes_behind_a_mark_and_mixed_line_endings() {
    let source = "\u{feff}# Title\r\n\r\nbody\rmore\r\n";
    assert_eq!(
        laid_in(Encoding::Utf8Bom, source),
        vec![
            Block::Heading {
                span: at(source, "# Title"),
                level: 1,
                inline: vec![text("Title")],
            },
            paragraph(
                source,
                "body\rmore",
                vec![text("body"), Inline::SoftBreak, text("more")]
            ),
        ]
    );
}

/// Strikethrough, emphasis beside CJK punctuation, a bare address, task
/// items and a footnote left where it was written.
#[test]
fn the_project_extensions_read_as_their_own_nodes() {
    let source = "~~gone~~ 这是**「重点」**的意思 https://example.org\n\n- [x] done\n- [ ] open\n\nNote[^1]\n\n[^1]: The note.\n";
    assert_eq!(
        laid(source),
        vec![
            paragraph(
                source,
                "~~gone~~ 这是**「重点」**的意思 https://example.org",
                vec![
                    Inline::Strikethrough(vec![text("gone")]),
                    text(" 这是"),
                    Inline::Strong(vec![text("「重点」")]),
                    text("的意思 "),
                    Inline::Link {
                        target: "https://example.org".to_owned(),
                        title: String::new(),
                        content: vec![text("https://example.org")],
                    },
                ]
            ),
            Block::List {
                span: at(source, "- [x] done\n- [ ] open"),
                order: Order::Bullet,
                spacing: Spacing::Tight,
                items: vec![
                    item(
                        source,
                        "- [x] done",
                        Check::Done,
                        vec![paragraph(source, "done", vec![text("done")])]
                    ),
                    item(
                        source,
                        "- [ ] open",
                        Check::Open,
                        vec![paragraph(source, "open", vec![text("open")])]
                    ),
                ],
            },
            paragraph(
                source,
                "Note[^1]",
                vec![
                    text("Note"),
                    Inline::FootnoteReference {
                        name: "1".to_owned()
                    }
                ]
            ),
            Block::Footnote {
                span: at(source, "[^1]: The note."),
                name: "1".to_owned(),
                blocks: vec![paragraph(source, "The note.", vec![text("The note.")])],
            },
        ]
    );
}

/// Front matter, an HTML block, a formula and inline tags are read, not
/// drawn, and each keeps the text the source gave it.
#[test]
fn what_the_grammar_reads_but_does_not_draw_keeps_its_source() {
    let source = "---\ntitle: x\n---\n\n<div>raw</div>\n\nArea $\\pi r^2$ and <b>bold</b>\n";
    assert_eq!(
        laid(source),
        vec![
            Block::Unsupported {
                span: at(source, "---\ntitle: x\n---"),
                construct: Construct::FrontMatter,
                source: "---\ntitle: x\n---".to_owned(),
            },
            Block::Unsupported {
                span: at(source, "<div>raw</div>"),
                construct: Construct::Html,
                source: "<div>raw</div>".to_owned(),
            },
            paragraph(
                source,
                "Area $\\pi r^2$ and <b>bold</b>",
                vec![
                    text("Area "),
                    Inline::Unsupported {
                        construct: Construct::Math,
                        source: "$\\pi r^2$".to_owned(),
                    },
                    text(" and "),
                    Inline::Unsupported {
                        construct: Construct::Html,
                        source: "<b>".to_owned(),
                    },
                    text("bold"),
                    Inline::Unsupported {
                        construct: Construct::Html,
                        source: "</b>".to_owned(),
                    },
                ]
            ),
        ]
    );
}

/// A link to a scheme the grammar does not admit keeps its words, and an
/// image of one keeps its alternative text.
#[test]
fn a_link_the_grammar_does_not_admit_keeps_its_words() {
    let source = "[a](javascript:alert(1)) [b](https://x.org \"B\") ![c](data:x) ![d](./d.png)";
    assert_eq!(
        laid(source),
        vec![paragraph(
            source,
            source,
            vec![
                text("a"),
                text(" "),
                Inline::Link {
                    target: "https://x.org".to_owned(),
                    title: "B".to_owned(),
                    content: vec![text("b")],
                },
                text(" "),
                text("c"),
                text(" "),
                Inline::Image {
                    target: "./d.png".to_owned(),
                    title: String::new(),
                    alt: "d".to_owned(),
                },
            ]
        )]
    );
}

/// Targets are judged as a browser reads an `href`.
#[test]
fn a_target_is_judged_as_a_browser_reads_it() {
    let targets = [
        "https://x.org",
        "HTTP://x.org",
        "mailto:someone@x.org",
        "./doc.md",
        "#top",
        "a/b:c",
        "javascript:alert(1)",
        "JaVaScRiPt:alert(1)",
        " \u{1}javascript:alert(1)",
        "java\tscript:alert(1)",
        "data:image/png;base64,AA",
        "file:///etc/hosts",
        "C:\\Windows",
    ];
    let judged: Vec<(&str, bool)> = targets.iter().map(|t| (*t, admitted(t))).collect();
    assert_eq!(
        judged,
        vec![
            ("https://x.org", true),
            ("HTTP://x.org", true),
            ("mailto:someone@x.org", true),
            ("./doc.md", true),
            ("#top", true),
            ("a/b:c", true),
            ("javascript:alert(1)", false),
            ("JaVaScRiPt:alert(1)", false),
            (" \u{1}javascript:alert(1)", false),
            ("java\tscript:alert(1)", false),
            ("data:image/png;base64,AA", false),
            ("file:///etc/hosts", false),
            ("C:\\Windows", false),
        ]
    );
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
        Preview::Laid {
            span: Span::at(0),
            blocks: Vec::new(),
        }
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

/// Seventeen nested quotes: sixteen are drawn, the seventeenth and all it
/// holds is shown as its source.
#[test]
fn a_quote_deeper_than_the_cap_is_shown_as_its_source() {
    let source = format!("{} deep\n", ">".repeat(40));
    let end = 45;
    let mut expected = Block::Unsupported {
        span: Span::new(16, end).unwrap(),
        construct: Construct::Nesting,
        source: format!("{} deep", ">".repeat(24)),
    };
    for depth in (0..16).rev() {
        expected = Block::Quote {
            span: Span::new(depth, end).unwrap(),
            blocks: vec![expected],
        };
    }
    assert_eq!(super::lowering::NESTING_MAX, 16);
    assert_eq!(laid(&source), vec![expected]);
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
        let Preview::Laid { span, blocks } = preview(
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
            let last = blocks.last().map(|block| match block {
                Block::Paragraph { span, .. } => span.end(),
                other => panic!("{other:?}"),
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
        Preview::Laid {
            span: Span::new(0, max).unwrap(),
            blocks: vec![Block::Paragraph {
                span: Span::new(0, max).unwrap(),
                inline: vec![text(&window)],
            }],
        }
    );
}
