// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The grammar's corpus. The CommonMark cases are the ones
//! `client/src/core/prose.test.ts` pinned for the page's own reader; the
//! rest are the extensions this grammar turns on, the source kept by
//! what it does not draw, and the nesting cap (D22, D24, D26). Every
//! block is compared whole, with the bytes of the version it names.

use super::*;

/// The blocks of `source` read as a whole version.
fn laid(source: &str) -> Vec<Block> {
    blocks(source, 0)
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
        laid(source),
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
    assert_eq!(NESTING_MAX, 16);
    assert_eq!(laid(&source), vec![expected]);
}
