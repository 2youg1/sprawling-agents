// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a cut says, and where a cut may land.
//!
//! Four places in this crate remove bytes from a text: the frozen
//! prefix, the result pipeline, the compactor and the sieve. They share
//! two facts, and both live here.
//!
//! **One sentence for "bytes were taken away".** A reader — a person or
//! a model — meets the same wording wherever it happens, and
//! `replay::rebuild_prefix` reproduces a recorded prefix byte for byte
//! because the sentence has one author. The marker is English and
//! ASCII: it faces the model's English window.
//!
//! **One rule for where a cut lands.** A byte budget and a cut in the
//! middle of a character produce bytes nothing downstream can read, so
//! every cut moves to a character boundary by the two functions below.
//!
//! The count a marker carries is **source bytes removed**, and the
//! marker itself is never one of them: add `dropped` to the length of
//! the text without its marker and the input length comes back.

use kernel::ByteLen;

/// Where the bytes that are gone used to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elided {
    /// Nothing was removed, so the text carries no marker.
    Nothing,
    /// The front is gone and the marker opens the text.
    Head,
    /// The middle is gone and the marker sits on a line of its own.
    Middle,
    /// The tail is gone and the marker closes the text.
    Tail,
}

/// One shortened text, with the account of what it no longer carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut {
    pub text: String,
    /// Source bytes removed; the marker's own bytes are not counted.
    pub dropped: ByteLen,
    pub place: Elided,
}

impl Cut {
    /// The text as it arrived: nothing removed, nothing marked.
    #[must_use]
    pub fn whole(text: &str) -> Cut {
        Cut {
            text: text.to_owned(),
            dropped: ByteLen::new(0),
            place: Elided::Nothing,
        }
    }
}

/// The sentence that stands where the removed bytes were.
#[must_use]
pub fn marker(dropped: ByteLen) -> String {
    let dropped = dropped.get();
    format!("[truncated: {dropped} bytes]")
}

/// The same sentence on a line of its own, for bytes removed from
/// inside a text rather than from one of its ends.
#[must_use]
pub fn gap_marker(dropped: ByteLen) -> String {
    let marker = marker(dropped);
    format!("\n{marker}\n")
}

/// How many bytes to reserve for a marker before the count is known.
///
/// A cut cannot remove more bytes than the text holds, and the marker
/// grows only with the digits of its count, so the marker for a text's
/// whole length is the widest one that text can produce.
#[must_use]
pub fn marker_room(text_len: usize) -> usize {
    marker(saturating_len(text_len)).len()
}

/// `marker_room` for a marker on a line of its own.
#[must_use]
pub fn gap_marker_room(text_len: usize) -> usize {
    gap_marker(saturating_len(text_len)).len()
}

/// The largest character boundary at or before `at`.
#[must_use]
pub fn boundary_before(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at = at.saturating_sub(1);
    }
    at
}

/// The smallest character boundary at or after `at`.
#[must_use]
pub fn boundary_after(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while at < text.len() && !text.is_char_boundary(at) {
        at = at.saturating_add(1);
    }
    at
}

/// A length this machine can address, as a byte count this city can
/// hold. A text longer than `u64` cannot exist on a machine whose
/// pointers are 64 bits, so the saturation is unreachable rather than a
/// decision anybody has to review.
fn saturating_len(len: usize) -> ByteLen {
    ByteLen::new(u64::try_from(len).unwrap_or(u64::MAX))
}

/// The cut a caller assembles: the bytes before `front`, the marker,
/// and the bytes from `back` on.
///
/// `front` and `back` must be character boundaries with `front <= back`
/// — every caller here obtains them from `boundary_before` and
/// `boundary_after`. The removed count is the distance between them,
/// which is what makes the marker's number true by construction.
#[must_use]
pub fn splice(text: &str, front: usize, back: usize, place: Elided) -> Cut {
    let dropped = saturating_len(back.saturating_sub(front));
    let mut out = String::new();
    out.push_str(text.get(..front).unwrap_or_default());
    match place {
        Elided::Nothing => {}
        Elided::Head | Elided::Tail => out.push_str(&marker(dropped)),
        Elided::Middle => out.push_str(&gap_marker(dropped)),
    }
    out.push_str(text.get(back..).unwrap_or_default());
    Cut {
        text: out,
        dropped,
        place,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn the_count_in_the_marker_is_the_bytes_the_reader_lost() {
        let text = "0123456789";
        let cut = splice(text, 3, 8, Elided::Middle);
        assert_eq!(cut.dropped, ByteLen::new(5));
        assert_eq!(cut.text, format!("012{}89", gap_marker(ByteLen::new(5))));
        let carried = cut.text.len() - gap_marker(ByteLen::new(5)).len();
        assert_eq!(
            carried + 5,
            text.len(),
            "the marker is not a source byte, so the two add back up"
        );
    }

    #[test]
    fn reserved_room_is_never_less_than_the_marker_that_arrives() {
        for len in [0usize, 1, 9, 10, 4_096, 1_000_000] {
            let room = marker_room(len);
            for dropped in [0u64, 1, 9, 10] {
                let actual = marker(ByteLen::new(dropped.min(saturating_len(len).get())));
                assert!(actual.len() <= room, "a marker wider than its reservation");
            }
            assert!(gap_marker_room(len) > room, "the gap marker owns two lines");
        }
    }

    #[test]
    fn a_cut_never_lands_inside_a_character() {
        let text = "字字字";
        for at in 0..=text.len() {
            assert!(text.is_char_boundary(boundary_before(text, at)));
            assert!(text.is_char_boundary(boundary_after(text, at)));
        }
        assert_eq!(boundary_before(text, 100), text.len());
        assert_eq!(boundary_after(text, 100), text.len());
    }
}
