// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn cut<'a>(text: &'a str, span: &Span) -> &'a str {
    let start = usize::try_from(span.start).unwrap();
    let len = usize::try_from(span.len).unwrap();
    text.get(start..start + len)
        .expect("a span slices its text")
}

fn tokens(text: &str) -> Vec<(Token, &str)> {
    markdown(text)
        .iter()
        .map(|span| (span.token, cut(text, span)))
        .collect()
}

#[test]
fn a_heading_is_read_whole_including_its_hashes() {
    assert_eq!(
        tokens("# What happened\n"),
        vec![(Token::Heading, "# What happened")]
    );
}

#[test]
fn a_fence_names_its_language_and_holds_its_block_as_code() {
    let doc = "before\n```rust\nlet x = 1;\nlet y = 2;\n```\nafter\n";
    let read = tokens(doc);
    assert_eq!(read[0], (Token::Fence, "```"));
    assert_eq!(read[1], (Token::Meta, "rust"));
    assert_eq!(read[2], (Token::Code, "let x = 1;\nlet y = 2;\n"));
    assert_eq!(read[3], (Token::Fence, "```"));
}

/// This build has no grammar engine, so reading `**x**` inside a
/// block of Rust as bold text would be inventing a fact.
#[test]
fn nothing_inside_a_fence_is_read_as_prose() {
    let doc = "```rust\nlet a = b ** c; // *not* emphasis\n```\n";
    let read = tokens(doc);
    assert!(
        read.iter()
            .all(|(token, _)| matches!(token, Token::Fence | Token::Meta | Token::Code)),
        "{read:?}"
    );
}

/// A truncated document still ends somewhere, and its tail is still
/// code. `BuildingDoc` carries a `truncated` flag, so this arrives.
#[test]
fn a_fence_nobody_closed_still_holds_the_rest_of_the_document() {
    let doc = "```\nhalf a program\n";
    let read = tokens(doc);
    assert_eq!(read[0].0, Token::Fence);
    assert_eq!(read[1], (Token::Code, "half a program\n"));
}

#[test]
fn a_bullet_is_a_marker_and_a_star_around_a_word_is_not() {
    assert_eq!(
        tokens("- *ready*\n"),
        vec![(Token::Marker, "- "), (Token::Emphasis, "*ready*")]
    );
    assert_eq!(tokens("*ready*\n"), vec![(Token::Emphasis, "*ready*")]);
}

/// A backtick outranks a star: a code span holding stars is code, not
/// bold text inside code.
#[test]
fn a_code_span_wins_the_bytes_it_covers() {
    assert_eq!(
        tokens("call `a ** b` twice\n"),
        vec![(Token::Code, "`a ** b`")]
    );
}

#[test]
fn strong_is_not_read_as_two_emphases() {
    assert_eq!(tokens("**loud**\n"), vec![(Token::Strong, "**loud**")]);
}

#[test]
fn a_link_is_read_whole_so_a_reader_can_see_where_it_goes() {
    assert_eq!(
        tokens("see [the plan](lab/Roadmap.md) first\n"),
        vec![(Token::Link, "[the plan](lab/Roadmap.md)")]
    );
}

#[test]
fn a_quote_marker_is_its_own_span_and_its_line_is_still_read() {
    assert_eq!(
        tokens("> **note**\n"),
        vec![(Token::Quote, ">"), (Token::Strong, "**note**")]
    );
}

/// The property the interface depends on: every span slices, and no
/// two claim the same byte. A page walking them beside the text would
/// otherwise have to decide which claim wins, which is a lexical rule
/// leaking into a view.
#[test]
fn every_span_slices_its_text_and_none_of_them_overlap() {
    let doc = "# 标题\n\n段落里有 `代码` 与 **重点**，还有 [链接](到/别处.md)。\n\n\
               - 第一条 *强调*\n- 第二条\n\n```rust\nlet 值 = 1;\n```\n\n> 引用一句\n";
    let spans = markdown(doc);
    assert!(!spans.is_empty());
    let mut end = 0u32;
    for span in &spans {
        assert!(
            span.start >= end,
            "spans overlap at {}: {spans:?}",
            span.start
        );
        // Panics on a boundary that is not a character boundary,
        // which is the failure a Chinese document would produce.
        let _ = cut(doc, span);
        end = span.start.saturating_add(span.len);
    }
}

#[test]
fn an_empty_document_reads_as_nothing_rather_than_as_one_empty_span() {
    assert!(markdown("").is_empty());
    assert!(markdown("\n\n").is_empty());
}

/// Punctuation somebody typed is not a span with no content.
#[test]
fn an_empty_pair_of_marks_is_not_a_span() {
    assert!(markdown("nothing `` here\n").is_empty());
}
