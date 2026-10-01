// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn edit(start: u64, end: u64, bytes: &[u8]) -> Edit {
    Edit {
        span: Span::new(start, end).unwrap(),
        bytes: bytes.to_vec(),
    }
}

fn on(source: &[u8], edits: Vec<Edit>) -> Transaction {
    Transaction::new(B3Hash::digest(source), edits).unwrap()
}

/// The A1 samples: a mark, mixed line endings, trailing spaces, and
/// bytes that are not UTF-8.
const SAMPLES: [&[u8]; 4] = [
    b"\xef\xbb\xbf# title\n\nbody\n",
    b"one\r\ntwo\nthree\r\n\r\nfour",
    b"trailing   \nspaces\t \n",
    b"caf\xe9 au lait\n\xff\xfe\x00 raw\n",
];

/// A1: a save changes the bytes it names and copies every other byte
/// through as it was.
#[test]
fn every_byte_outside_the_edits_is_copied_through() {
    for source in SAMPLES {
        let last = offset(source.len());
        let saved = on(
            source,
            vec![
                edit(1, 2, b"X"),
                edit(4, 4, b"++"),
                edit(last - 1, last, b""),
            ],
        )
        .apply(source)
        .unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(&source[..1]);
        expected.extend_from_slice(b"X");
        expected.extend_from_slice(&source[2..4]);
        expected.extend_from_slice(b"++");
        expected.extend_from_slice(&source[4..source.len() - 1]);
        assert_eq!(saved.bytes(), expected.as_slice());
        assert_eq!(saved.version(), B3Hash::digest(&expected));
    }
}

/// D9: what a save hands back takes the new version back to the old,
/// byte for byte, and is made on the new version.
#[test]
fn the_undo_a_save_hands_back_restores_the_baseline() {
    for source in SAMPLES {
        let saved = on(source, vec![edit(0, 3, b""), edit(5, 6, "段落".as_bytes())])
            .apply(source)
            .unwrap();
        assert_eq!(saved.undo().baseline(), saved.version());
        let undone = saved.undo().apply(saved.bytes()).unwrap();
        assert_eq!(undone.bytes(), source);
        assert_eq!(undone.version(), B3Hash::digest(source));
    }
}

/// A3: two writers start from one version with different changes; the
/// first lands, and the second is refused rather than written over it.
#[test]
fn a_second_save_from_the_same_version_is_refused() {
    let source = b"# Roadmap\n\n- one\n";
    let first = on(source, vec![edit(13, 16, b"first")]);
    let second = on(source, vec![edit(13, 16, b"second")]);
    let landed = first.apply(source).unwrap();
    let refused = second.apply(landed.bytes()).unwrap_err();
    assert_eq!(refused.code(), &AxCode::VersionConflict);
    assert_eq!(landed.bytes(), b"# Roadmap\n\n- first\n");
}

#[test]
fn edits_that_share_a_byte_are_refused_and_two_insertions_keep_their_order() {
    let source = b"abcdef";
    let refused = Transaction::new(
        B3Hash::digest(source),
        vec![edit(1, 4, b"x"), edit(3, 5, b"y")],
    )
    .unwrap_err();
    assert_eq!(refused.code(), &AxCode::InvalidArgs);
    let saved = on(source, vec![edit(2, 2, b"1"), edit(2, 2, b"2")])
        .apply(source)
        .unwrap();
    assert_eq!(saved.bytes(), b"ab12cdef");
}

#[test]
fn an_edit_past_the_end_is_refused_and_nothing_lands() {
    let source = b"abc";
    let refused = on(source, vec![edit(1, 9, b"x")])
        .apply(source)
        .unwrap_err();
    assert_eq!(refused.code(), &AxCode::InvalidArgs);
}

fn text_edit(start: u64, end: u64, text: &str) -> TextEdit {
    TextEdit {
        span: Span::new(start, end).unwrap(),
        text: text.to_owned(),
    }
}

/// D11: a page's text lands in the encoding the version is in, the
/// mark and every byte outside the edit kept.
#[test]
fn a_text_save_writes_the_version_s_own_encoding() {
    let cases: [(&[u8], TextEdit, &[u8]); 4] = [
        (b"ab", text_edit(1, 2, "\u{4e2d}"), "a\u{4e2d}".as_bytes()),
        (b"\xef\xbb\xbfx", text_edit(3, 4, "y"), b"\xef\xbb\xbfy"),
        (
            b"\xff\xfea\x00b\x00",
            text_edit(4, 6, "\u{4e2d}"),
            b"\xff\xfea\x00\x2d\x4e",
        ),
        (
            b"\xfe\xff\x00a\x00b",
            text_edit(2, 4, "\u{4e2d}"),
            b"\xfe\xff\x4e\x2d\x00b",
        ),
    ];
    for (source, edit, expected) in cases {
        let saved = save(source, B3Hash::digest(source), &[edit]).unwrap();
        assert_eq!(saved.bytes(), expected);
        assert_eq!(saved.version(), B3Hash::digest(expected));
        assert_eq!(saved.baseline(), B3Hash::digest(source));
    }
}

/// D12: an empty version, or a file that is not there yet, is the
/// baseline its first version is saved on.
#[test]
fn a_save_on_an_empty_version_writes_its_first_bytes() {
    let saved = save(b"", B3Hash::digest(b""), &[text_edit(0, 0, "hello\n")]).unwrap();
    assert_eq!(saved.bytes(), b"hello\n");
}

/// D11, D12: a save that would leave bytes another reading, or that
/// starts inside a character, is refused before anything lands, and a
/// version that moved is told so first.
#[test]
fn a_save_that_splits_a_character_or_leaves_no_text_is_refused() {
    let accented = "caf\u{e9}".as_bytes();
    let refusals: [(&[u8], TextEdit, &str); 5] = [
        (accented, text_edit(4, 5, "e"), "E_INVALID_ARGS"),
        (b"plain", text_edit(1, 1, "\0"), "E_INVALID_ARGS"),
        (b"\xef\xbb\xbfx", text_edit(0, 3, ""), "E_INVALID_ARGS"),
        (
            b"\xff\xfea\x00b\x00",
            text_edit(3, 5, "c"),
            "E_INVALID_ARGS",
        ),
        (b"\x00raw", text_edit(0, 1, "x"), "E_INVALID_ARGS"),
    ];
    for (source, edit, code) in refusals {
        let refused = save(source, B3Hash::digest(source), &[edit]).unwrap_err();
        assert_eq!(refused.code().as_str(), code, "{source:?}");
    }
    let moved = save(
        b"\x00raw",
        B3Hash::digest(b"other"),
        &[text_edit(0, 1, "x")],
    )
    .unwrap_err();
    assert_eq!(moved.code().as_str(), "E_VERSION_CONFLICT");
}
