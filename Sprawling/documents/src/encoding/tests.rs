// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn utf16_le(text: &str) -> Vec<u8> {
    let mut bytes = UTF16_LE_MARK.to_vec();
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

fn utf16_be(text: &str) -> Vec<u8> {
    let mut bytes = UTF16_BE_MARK.to_vec();
    bytes.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
    bytes
}

/// A UTF-16 file holds a NUL beside every ASCII character; the mark in
/// front is what says it is text, and the NULs do not unsay it.
#[test]
fn utf16_behind_its_mark_is_text_whatever_nul_it_holds() {
    let little = utf16_le("plain words\r\n");
    assert!(little.contains(&0));
    assert_eq!(Reading::of(&little), Reading::Text(Encoding::Utf16Le));
    assert_eq!(
        Reading::of(&utf16_be("段落")),
        Reading::Text(Encoding::Utf16Be)
    );
}

#[test]
fn a_nul_without_a_mark_is_not_text() {
    assert_eq!(Reading::of(b"redb\0\x01\x02"), Reading::Opaque);
}

#[test]
fn bytes_that_are_not_utf8_and_carry_no_mark_are_not_text() {
    // "caf\xe9" is Latin-1; read as UTF-8 it would be a replacement
    // character, a character the file does not hold.
    assert_eq!(Reading::of(b"caf\xe9 au lait\n"), Reading::Opaque);
}

#[test]
fn utf8_reads_with_and_without_its_mark() {
    assert_eq!(
        Reading::of("第一段。\n".as_bytes()),
        Reading::Text(Encoding::Utf8)
    );
    assert_eq!(
        Reading::of("\u{feff}第一段。\n".as_bytes()),
        Reading::Text(Encoding::Utf8Bom)
    );
    assert_eq!(Reading::of(b""), Reading::Text(Encoding::Utf8));
}

/// A lone high surrogate is not a character, so a UTF-16 file that
/// carries one is not text.
#[test]
fn utf16_with_a_lone_surrogate_is_not_text() {
    let mut bytes = UTF16_LE_MARK.to_vec();
    bytes.extend(0xD800_u16.to_le_bytes());
    assert_eq!(Reading::of(&bytes), Reading::Opaque);
}

/// D5: the mark is the first character of the text, so what a reader is
/// given encodes back to exactly the bytes it came from.
#[test]
fn decoded_text_encodes_back_to_its_own_bytes() {
    let marked = "\u{feff}行尾双空格  \r\n接续行。\n";
    assert_eq!(Encoding::Utf8Bom.decode(marked.as_bytes()).unwrap(), marked);
    let little = utf16_le("a\u{1d11e}b");
    let text = Encoding::Utf16Le.decode(&little).unwrap();
    assert_eq!(text, "\u{feff}a\u{1d11e}b");
    assert_eq!(utf16_le(text.trim_start_matches('\u{feff}')), little);
}

#[test]
fn utf8_decoding_refuses_a_nul_that_the_marked_form_allows() {
    let refused = Encoding::Utf8.decode(b"a\0b").unwrap_err();
    assert_eq!(refused.code(), &AxCode::InvalidArgs);
    assert_eq!(Encoding::Utf8Bom.decode(b"a\0b").unwrap(), "a\0b");
}

#[test]
fn a_character_begins_where_no_continuation_byte_or_low_surrogate_stands() {
    let bytes = "a段".as_bytes();
    assert!(Encoding::Utf8.begins_at(bytes, 0, 0));
    assert!(Encoding::Utf8.begins_at(bytes, 1, 0));
    assert!(!Encoding::Utf8.begins_at(bytes, 2, 0));
    let little = utf16_le("\u{1d11e}");
    // The mark, then a high surrogate, then its low surrogate.
    assert!(Encoding::Utf16Le.begins_at(&little, 2, 0));
    assert!(!Encoding::Utf16Le.begins_at(&little, 3, 0));
    assert!(!Encoding::Utf16Le.begins_at(&little, 4, 0));
    // The same bytes lifted from an odd offset are not aligned to a unit.
    assert!(!Encoding::Utf16Le.begins_at(&little, 2, 1));
}
