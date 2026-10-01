// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
// Portions copyright (c) 2026 2youg1 and the RefRain contributors

use super::*;

/// RefRain's regression corpus for its source layout: each entry a shape
/// that has damaged, or could damage, an author's file, and the block
/// spans RefRain froze for it. The texts are the ones its manifest pins
/// by SHA-256, carried here as literals because RefRain generates the
/// files rather than keeping them.
const CORPUS: &[(&str, &str, &[(u64, u64)])] = &[
    (
        "ideographic-indent",
        "\u{3000}\u{3000}全角空格缩进的段落，中文写作常用。\n\n第二段也缩进。\n",
        &[(0, 57), (59, 80)],
    ),
    (
        "half-width-indent",
        "    four spaces open this line\n\nplain.\n",
        &[(0, 30), (32, 38)],
    ),
    (
        "consecutive-blank-lines",
        "一\n\n\n\n二\n\n\n三\n",
        &[(0, 3), (7, 10), (13, 16)],
    ),
    (
        "fence-holding-a-blank-line",
        "前言\n\n```ts\nconst a = 1;\n\nconst b = 2;\n```\n\n后记\n",
        &[(0, 6), (8, 44), (46, 52)],
    ),
    ("tilde-fence", "~~~\n\n~~~\n\n之后。\n", &[(0, 8), (10, 19)]),
    (
        "nested-fence-markers",
        "````md\n```ts\n\nconst a = 1;\n```\n````\n",
        &[(0, 35)],
    ),
    (
        "hard-line-break",
        "行尾双空格  \n接续行。\n\n第二段。\n",
        &[(0, 30), (32, 44)],
    ),
    (
        "crlf",
        "第一段。\r\n\r\n第二段。\r\n\r\n\r\n第三段。\r\n",
        &[(0, 12), (16, 28), (34, 46)],
    ),
    ("no-trailing-newline", "只有一段，末尾无换行", &[(0, 30)]),
    (
        "byte-order-mark",
        "\u{feff}第一段。\n\n第二段。\n",
        &[(0, 15), (17, 29)],
    ),
    ("leading-blank-lines", "\n\n\n开头前有空行。\n", &[(3, 24)]),
    ("trailing-blank-lines", "结尾后有空行。\n\n\n\n", &[(0, 21)]),
    (
        "astral-characters",
        "𝄞 音乐符号，非 BMP。\n\n😀 表情符号。\n",
        &[(0, 30), (32, 52)],
    ),
    (
        "mixed-scripts",
        "漢字とかなの段落。\n\n\u{3000}\u{3000}全角インデント。\n\nLatin paragraph.\n",
        &[(0, 27), (29, 59), (61, 77)],
    ),
    (
        "blockquote-and-list",
        "> 引用。\n> 续行。\n\n- 一\n- 二\n\n1. 甲\n2. 乙\n",
        &[(0, 23), (25, 36), (38, 51)],
    ),
    (
        "table",
        "| a | b |\n|---|---|\n| 1 | 2 |\n\n后文。\n",
        &[(0, 29), (31, 40)],
    ),
    (
        "tabs",
        "\t制表符开头。\n\n\t\t两个制表符。\n",
        &[(0, 19), (21, 41)],
    ),
    ("empty-file", "", &[]),
    ("only-whitespace", "\n\n   \n\n", &[]),
    (
        "everything-at-once",
        "# 标题\n\n\u{3000}\u{3000}全角空格缩进的段落。\n\n\n\n行尾双空格  \n接续行。\n\n```ts\nconst a = 1;\n\nconst b = 2;\n```\n\n    四空格缩进的码块\n\n> 引用。\n",
        &[
            (0, 8),
            (10, 46),
            (50, 80),
            (82, 118),
            (120, 148),
            (150, 161),
        ],
    ),
];

/// Blocks and the gaps between them, laid end to end.
fn reproduce(source: &[u8], spans: &[Span]) -> Vec<u8> {
    let mut out = Vec::with_capacity(source.len());
    let mut cursor = 0_usize;
    for span in spans {
        let (start, end) = span.within(source.len()).unwrap();
        out.extend_from_slice(&source[cursor..start]);
        out.extend_from_slice(&source[start..end]);
        cursor = end;
    }
    out.extend_from_slice(&source[cursor..]);
    out
}

#[test]
fn every_frozen_corpus_keeps_its_block_spans_and_its_bytes() {
    for (name, text, frozen) in CORPUS {
        let source = text.as_bytes();
        let spans = blocks(Format::Markdown, source);
        let found: Vec<(u64, u64)> = spans
            .iter()
            .map(|span| (span.start(), span.end()))
            .collect();
        assert_eq!(found, frozen.to_vec(), "{name}");
        assert_eq!(reproduce(source, &spans), source, "{name}");
    }
}

/// A1: a block never takes a line ending, a trailing space or a byte that
/// is not UTF-8 away from the bytes around it.
#[test]
fn plain_lines_reproduce_mixed_endings_trailing_spaces_and_foreign_bytes() {
    let source = b"one  \r\ntwo\n\r\ncaf\xe9\t \nlast";
    let spans = blocks(Format::Plain, source);
    let found: Vec<(u64, u64)> = spans
        .iter()
        .map(|span| (span.start(), span.end()))
        .collect();
    assert_eq!(found, vec![(0, 5), (7, 10), (11, 11), (13, 19), (20, 24)]);
    assert_eq!(reproduce(source, &spans), source);
    assert_eq!(reproduce(source, &blocks(Format::Markdown, source)), source);
}

#[test]
fn an_empty_plain_document_is_one_empty_line() {
    assert_eq!(blocks(Format::Plain, b""), vec![Span::at(0)]);
    assert_eq!(blocks(Format::Markdown, b""), Vec::<Span>::new());
}
