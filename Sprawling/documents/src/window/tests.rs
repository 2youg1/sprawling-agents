// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::encoding::Reading;

fn whole(source: &[u8]) -> Lifted<'_> {
    Lifted {
        at: 0,
        bytes: source,
        size: offset(source.len()),
    }
}

fn span(start: u64, end: u64) -> Span {
    Span::new(start, end).unwrap()
}

/// Reads a version the way a page does: the head, then one range after
/// another from where the last one ended, each lifted on its own.
fn read_through(format: Format, source: &[u8]) -> Vec<Window> {
    let Reading::Text(encoding) = Reading::of(source) else {
        panic!("the fixture is text");
    };
    let size = offset(source.len());
    let mut windows = vec![head(format, encoding, source).unwrap()];
    while let Some(last) = windows.last().filter(|window| window.span.end() < size) {
        let wanted = span(last.span.end(), size);
        let lift = lift(wanted, size);
        let (from, to) = lift.within(source.len()).unwrap();
        let lifted = Lifted {
            at: lift.start(),
            bytes: &source[from..to],
            size,
        };
        windows.push(cut(encoding, lifted, wanted).unwrap());
    }
    windows
}

/// A1 on the read path: windows laid end to end are the version, byte
/// for byte - the mark, mixed line endings and trailing spaces included -
/// and none of them is longer than one answer may be.
#[test]
fn windows_read_end_to_end_give_back_every_byte() {
    let paragraph = "行尾双空格  \r\n接续行。\n\n".repeat(4_000);
    let fixtures: Vec<(Format, Vec<u8>)> = vec![
        (
            Format::Markdown,
            format!("\u{feff}{paragraph}").into_bytes(),
        ),
        (
            Format::Plain,
            "one  \r\ntwo\n\r\n\tlast  ".repeat(9_000).into_bytes(),
        ),
        (Format::Plain, "段".repeat(50_000).into_bytes()),
        (Format::Plain, utf16_le(&"a\u{1d11e}b\r\n".repeat(20_000))),
    ];
    for (format, source) in fixtures {
        let windows = read_through(format, &source);
        assert!(windows.len() > 1, "{format:?} fits one window");
        let mut cursor = 0;
        let mut rebuilt = Vec::new();
        for window in &windows {
            assert_eq!(window.span.start(), cursor);
            assert!(window.span.len() <= WINDOW_BYTES_MAX);
            let (from, to) = window.span.within(source.len()).unwrap();
            rebuilt.extend_from_slice(&source[from..to]);
            cursor = window.span.end();
        }
        assert_eq!(rebuilt, source);
        let text: String = windows.iter().map(|window| window.text.as_str()).collect();
        let Reading::Text(encoding) = Reading::of(&source) else {
            panic!("text");
        };
        assert_eq!(text, encoding.decode(&source).unwrap());
    }
}

fn utf16_le(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

#[test]
fn a_version_that_fits_is_its_own_head() {
    let source = "\u{feff}# 标题\n\n正文  \r\n".as_bytes();
    let window = head(Format::Markdown, Encoding::Utf8Bom, source).unwrap();
    assert_eq!(window.span, span(0, offset(source.len())));
    assert_eq!(window.text.as_bytes(), source);
}

/// The first screen holds no half paragraph: the head of a long
/// Markdown file ends where a block does.
#[test]
fn a_long_head_ends_at_the_last_block_that_fits() {
    let block = "一段话。".repeat(100);
    let source = format!("{block}\n\n").repeat(200).into_bytes();
    let window = head(Format::Markdown, Encoding::Utf8, &source).unwrap();
    let ends: Vec<u64> = layout::blocks(Format::Markdown, &source)
        .iter()
        .map(|block| block.end())
        .collect();
    assert!(ends.contains(&window.span.end()), "{}", window.span.end());
    assert!(window.span.end() <= WINDOW_BYTES_MAX);
    assert!(window.span.end() > WINDOW_BYTES_MAX - offset(block.len()) - 2);
}

#[test]
fn a_start_inside_a_character_steps_back_to_it_and_an_end_never_splits_one() {
    let source = "a段b".as_bytes();
    let window = cut(Encoding::Utf8, whole(source), span(2, 3)).unwrap();
    assert_eq!(window.span, span(1, 1));
    assert_eq!(window.text, "");
    let window = cut(Encoding::Utf8, whole(source), span(2, 5)).unwrap();
    assert_eq!(window.span, span(1, 5));
    assert_eq!(window.text, "段b");
}

#[test]
fn a_utf16_window_keeps_to_whole_units_and_whole_pairs() {
    let source = utf16_le("\u{1d11e}x");
    // Mark 0..2, high surrogate 2..4, low 4..6, x 6..8.
    let window = cut(Encoding::Utf16Le, whole(&source), span(5, 8)).unwrap();
    assert_eq!(window.span, span(2, 8));
    assert_eq!(window.text, "\u{1d11e}x");
    let window = cut(Encoding::Utf16Le, whole(&source), span(0, 5)).unwrap();
    assert_eq!(window.span, span(0, 2));
    assert_eq!(window.text, "\u{feff}");
}

#[test]
fn a_window_asked_for_past_the_end_is_empty_at_the_end() {
    let source = b"short";
    let window = cut(Encoding::Utf8, whole(source), span(40, 90)).unwrap();
    assert_eq!(
        window,
        Window {
            span: span(5, 5),
            text: String::new(),
        }
    );
    assert_eq!(lift(span(40, 90), 5), span(2, 5));
}

#[test]
fn a_cut_from_bytes_that_do_not_cover_the_window_is_refused() {
    let lifted = Lifted {
        at: 10,
        bytes: b"abc",
        size: 100,
    };
    let refused = cut(Encoding::Utf8, lifted, span(0, 4)).unwrap_err();
    assert_eq!(refused.code(), &kernel::AxCode::InvalidArgs);
}
