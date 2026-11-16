// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::ByteLen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elided {

    Nothing,

    Head,

    Middle,

    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut {
    pub text: String,

    pub dropped: ByteLen,
    pub place: Elided,
}

impl Cut {

    #[must_use]
    pub fn whole(text: &str) -> Cut {
        Cut {
            text: text.to_owned(),
            dropped: ByteLen::new(0),
            place: Elided::Nothing,
        }
    }
}

#[must_use]
pub fn marker(dropped: ByteLen) -> String {
    let dropped = dropped.get();
    format!("[truncated: {dropped} bytes]")
}

#[must_use]
pub fn gap_marker(dropped: ByteLen) -> String {
    let marker = marker(dropped);
    format!("\n{marker}\n")
}

#[must_use]
pub fn marker_room(text_len: usize) -> usize {
    marker(saturating_len(text_len)).len()
}

#[must_use]
pub fn gap_marker_room(text_len: usize) -> usize {
    gap_marker(saturating_len(text_len)).len()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Boundary<'a> {
    head: &'a str,
    tail: &'a str,
}

impl<'a> Boundary<'a> {

    #[must_use]
    pub(crate) fn before(text: &'a str, at: usize) -> Boundary<'a> {
        let mut at = at.min(text.len());
        loop {

            if let Some((head, tail)) = text.split_at_checked(at) {
                return Boundary { head, tail };
            }
            at = at.saturating_sub(1);
        }
    }

    #[must_use]
    pub(crate) fn after(text: &'a str, at: usize) -> Boundary<'a> {
        let mut at = at.min(text.len());
        loop {

            if let Some((head, tail)) = text.split_at_checked(at) {
                return Boundary { head, tail };
            }
            at = at.saturating_add(1);
        }
    }

    #[must_use]
    pub(crate) fn head(self) -> &'a str {
        self.head
    }

    #[must_use]
    pub(crate) fn tail(self) -> &'a str {
        self.tail
    }

    #[must_use]
    pub(crate) fn offset(self) -> usize {
        self.head.len()
    }
}

#[must_use]
pub fn boundary_before(text: &str, at: usize) -> usize {
    Boundary::before(text, at).offset()
}

#[must_use]
pub fn boundary_after(text: &str, at: usize) -> usize {
    Boundary::after(text, at).offset()
}

fn saturating_len(len: usize) -> ByteLen {
    ByteLen::new(u64::try_from(len).unwrap_or(u64::MAX))
}

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

    #[test]
    fn a_cut_keeps_the_bytes_it_was_given_at_any_offset() {
        let text = "字字字";
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
