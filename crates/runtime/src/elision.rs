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
//! **One rule for where a cut lands, and it answers with a value.** A
//! byte budget and a cut in the middle of a character produce bytes
//! nothing downstream can read, so every cut moves to a character
//! boundary, and the answer is a `Boundary`: the bytes on each side
//! of the cut, each holding whole characters. An offset and the text it
//! indexes are two facts that can disagree, and `str::get` answers the
//! disagreement with `None`, which a caller then reads as "nothing was
//! kept here"; the two sides cannot disagree with the text they came
//! from.
//!
//! The count a marker carries is **source bytes removed**, and the
//! marker itself is never one of them: add `dropped` to the length of
//! the text without its marker and the input length comes back, at any
//! offset a caller computed.

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

/// Where a cut may land in a text: the bytes before the cut and the
/// bytes from it, each holding whole characters.
///
/// A position is this value and never a bare offset, because the two
/// sides cannot disagree with the text they were taken from while an
/// offset and its text can. The constructors ask the text for the split
/// itself — the same question a caller would answer with
/// `str::is_char_boundary` — so the text's answer is the only one, and
/// a cut that would land inside a character cannot be constructed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Boundary<'a> {
    head: &'a str,
    tail: &'a str,
}

impl<'a> Boundary<'a> {
    /// The last position at or before `at`.
    #[must_use]
    pub(crate) fn before(text: &'a str, at: usize) -> Boundary<'a> {
        let mut at = at.min(text.len());
        loop {
            // Zero is a boundary of every text, and `at` walks down to
            // it, so this search ends.
            if let Some((head, tail)) = text.split_at_checked(at) {
                return Boundary { head, tail };
            }
            at = at.saturating_sub(1);
        }
    }

    /// The first position at or after `at`.
    #[must_use]
    pub(crate) fn after(text: &'a str, at: usize) -> Boundary<'a> {
        let mut at = at.min(text.len());
        loop {
            // The end of every text is a boundary, and `at` climbs to
            // it, so this search ends.
            if let Some((head, tail)) = text.split_at_checked(at) {
                return Boundary { head, tail };
            }
            at = at.saturating_add(1);
        }
    }

    /// The bytes before the cut.
    #[must_use]
    pub(crate) fn head(self) -> &'a str {
        self.head
    }

    /// The bytes from the cut on.
    #[must_use]
    pub(crate) fn tail(self) -> &'a str {
        self.tail
    }

    /// How far the cut is from the start of the text, in bytes.
    #[must_use]
    pub(crate) fn offset(self) -> usize {
        self.head.len()
    }
}

/// The largest character boundary at or before `at`.
///
/// `Boundary::before` is this offset's one author; a caller that wants
/// the bytes on either side rather than the offset asks there.
#[must_use]
pub fn boundary_before(text: &str, at: usize) -> usize {
    Boundary::before(text, at).offset()
}

/// The smallest character boundary at or after `at`.
#[must_use]
pub fn boundary_after(text: &str, at: usize) -> usize {
    Boundary::after(text, at).offset()
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
/// Both ends move to a character boundary — back for `front`, forward
/// for `back` — and the far end is never left behind the near one. The
/// removed count is then the distance between two positions the text
/// itself reported, which is what makes the marker's number true by
/// construction, and why the count and the survivors always add back up
/// into the input.
#[must_use]
pub fn splice(text: &str, front: usize, back: usize, place: Elided) -> Cut {
    let back = back.max(front);
    let from = Boundary::before(text, front);
    let to = Boundary::after(text, back);
    let dropped = saturating_len(to.offset().saturating_sub(from.offset()));
    let mut out = String::new();
    out.push_str(from.head());
    match place {
        Elided::Nothing => {}
        Elided::Head | Elided::Tail => out.push_str(&marker(dropped)),
        Elided::Middle => out.push_str(&gap_marker(dropped)),
    }
    out.push_str(to.tail());
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
    use proptest::prelude::*;

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

    /// A caller that computed an offset without asking for a boundary
    /// must not lose bytes the marker does not count: the two numbers a
    /// cut reports are the input split in two.
    #[test]
    fn a_cut_keeps_the_bytes_it_was_given_at_any_offset() {
        let text = "字字字"; // three characters, nine bytes
        for front in 0..=text.len() {
            for back in front..=text.len() {
                let cut = splice(text, front, back, Elided::Tail);
                let carried = cut.text.len() - marker(cut.dropped).len();
                assert_eq!(
                    carried + usize::try_from(cut.dropped.get()).unwrap_or(usize::MAX),
                    text.len(),
                    "front {front} back {back} lost {} bytes and reported {}",
                    text.len() - carried,
                    cut.dropped.get()
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// The same statement over every text and every pair of offsets,
        /// including a pair a caller inverted.
        #[test]
        fn the_count_and_the_survivors_add_back_up_to_the_input(
            text in "[ -~\u{1b}字]{0,80}",
            front in 0usize..300,
            back in 0usize..300,
            place in prop::sample::select(vec![
                Elided::Nothing, Elided::Head, Elided::Middle, Elided::Tail,
            ]),
        ) {
            let cut = splice(&text, front, back, place);
            let own = match place {
                Elided::Middle => gap_marker(cut.dropped),
                Elided::Nothing => String::new(),
                Elided::Head | Elided::Tail => marker(cut.dropped),
            };
            let carried = cut.text.len().saturating_sub(own.len());
            prop_assert_eq!(
                carried + usize::try_from(cut.dropped.get()).unwrap_or(usize::MAX),
                text.len()
            );
        }
    }
}
