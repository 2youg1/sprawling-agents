// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn accept(slice: u32) -> SliceVerdict {
    SliceVerdict {
        slice,
        verdict: Verdict::Accept,
    }
}

fn amend(slice: u32, text: &str) -> SliceVerdict {
    SliceVerdict {
        slice,
        verdict: Verdict::Amend {
            text: text.to_owned(),
        },
    }
}

fn side(review: &Review, left_out: SliceKind) -> String {
    let mut out = String::new();
    for slice in review
        .slices()
        .iter()
        .filter(|slice| slice.kind != left_out)
    {
        slice.bytes_onto(&slice.text, &mut out);
    }
    out
}

fn changed(review: &Review) -> Vec<SliceVerdict> {
    (0_u32..)
        .zip(review.slices())
        .filter(|(_, slice)| slice.kind != SliceKind::Same)
        .map(|(place, _)| accept(place))
        .collect()
}

/// The stretch and the suggestion travel together: one change.
fn offer(run: u8, source: &str, change: (usize, usize, &str)) -> Offer {
    let (start, end, after) = change;
    Offer::of(
        RunId::from_bytes([run; 16]),
        &ProposalOffered {
            doc: Address::parse("lab/notes.md").unwrap(),
            baseline: B3Hash::digest(source.as_bytes()),
            start: u64::try_from(start).unwrap(),
            end: u64::try_from(end).unwrap(),
            before: source[start..end].to_owned(),
            after: after.to_owned(),
        },
    )
    .unwrap()
}

/// Pairs that cover the cut: sentences with and without a terminator,
/// closers after one, whitespace before, between and after, a side of
/// whitespace alone, an empty side, and a change of whitespace only.
const PAIRS: [(&str, &str); 8] = [
    ("甲。乙。", "甲。丙。"),
    (
        "He said \"go.\" Then left.  \n",
        "He said \"stay.\" Then left.\n",
    ),
    ("no terminator at all", "no terminator, still"),
    ("   ", "One sentence."),
    ("", "Written from nothing."),
    ("A.  B.", "A. B."),
    ("第一句。中间的一句。", "中间的一句。第一句。"),
    ("「走。」他说。\n\n下一段！", "「走吧。」他说。\n\n下一段？"),
];

/// D14: the slices of either side read back that side byte for byte,
/// so a card accepted whole is the suggestion and rejected whole is the
/// original.
#[test]
fn the_slices_of_either_side_read_back_that_side() {
    for (before, after) in PAIRS {
        let review = Review::of(before, after);
        assert_eq!(side(&review, SliceKind::Insert), before, "{before:?}");
        assert_eq!(side(&review, SliceKind::Delete), after, "{after:?}");
        assert_eq!(review.merged(&[]).unwrap(), before);
        assert_eq!(review.merged(&changed(&review)).unwrap(), after);
    }
}

/// D14: a change of whitespace alone is a change on the card, where
/// RefRain's trimmed key aligned it as the same sentence and lost it.
#[test]
fn a_change_of_whitespace_alone_is_on_the_card() {
    let review = Review::of("A.  B.", "A. B.");
    let kinds: Vec<SliceKind> = review.slices().iter().map(|slice| slice.kind).collect();
    assert_eq!(
        kinds,
        [SliceKind::Same, SliceKind::Delete, SliceKind::Insert]
    );
}

/// D16: a sentence taken, a sentence rewritten and a sentence left out,
/// on one card; the rewritten one keeps its whitespace.
#[test]
fn verdicts_take_rewrite_and_leave_out_sentences() {
    let review = Review::of("One. Two. Three.", "One. Deux. Three. Four.");
    let kinds: Vec<(SliceKind, &str)> = review
        .slices()
        .iter()
        .map(|slice| (slice.kind, slice.text.as_str()))
        .collect();
    assert_eq!(
        kinds,
        [
            (SliceKind::Same, "One."),
            (SliceKind::Delete, "Two."),
            (SliceKind::Insert, "Deux."),
            (SliceKind::Same, "Three."),
            (SliceKind::Insert, "Four."),
        ]
    );
    let merged = review
        .merged(&[accept(1), amend(2, "Zwei."), amend(4, "Vier.")])
        .unwrap();
    assert_eq!(merged, "One. Zwei. Three. Vier.");
    assert_eq!(
        review.merged(&[accept(4)]).unwrap(),
        "One. Two. Three. Four."
    );
}

/// D16: verdicts that name no changed sentence, name one twice, or amend
/// a deletion are refused.
#[test]
fn a_verdict_off_the_changed_sentences_is_refused() {
    let review = Review::of("One. Two.", "One. Deux.");
    for verdicts in [
        vec![accept(0)],
        vec![accept(9)],
        vec![accept(1), accept(1)],
        vec![amend(1, "Zwei.")],
    ] {
        let refused = review.merged(&verdicts).unwrap_err();
        assert_eq!(refused.code().as_str(), "E_INVALID_ARGS", "{verdicts:?}");
    }
}

/// D15: two sides with nothing in common past the table's budget are
/// one deletion and one insertion, and still read back both sides.
#[test]
fn sides_past_the_alignment_budget_are_replaced_whole() {
    let before: String = (0..2100).map(|n| format!("甲{n}。")).collect();
    let after: String = (0..2100).map(|n| format!("乙{n}。")).collect();
    let review = Review::of(&before, &after);
    let deleted = review
        .slices()
        .iter()
        .filter(|slice| slice.kind == SliceKind::Delete)
        .count();
    assert_eq!(deleted, 2100);
    assert_eq!(side(&review, SliceKind::Delete), after);
}

/// D13: the identity reads the same from the same line, and differs for
/// another run offering the same words.
#[test]
fn the_identity_is_the_offer_and_its_run() {
    let source = "One. Two.";
    assert_eq!(
        offer(1, source, (5, 9, "Deux.")).id(),
        offer(1, source, (5, 9, "Deux.")).id()
    );
    assert_ne!(
        offer(1, source, (5, 9, "Deux.")).id(),
        offer(2, source, (5, 9, "Deux.")).id()
    );
    assert_ne!(
        offer(1, source, (5, 9, "Deux.")).id(),
        offer(1, source, (5, 9, "Zwei.")).id()
    );
}

/// D18: an original or a suggestion longer than one window, and a
/// reversed stretch, are not proposals.
#[test]
fn an_offer_past_a_window_or_reversed_is_refused() {
    let long = "x".repeat(usize::try_from(WINDOW_BYTES_MAX).unwrap() + 1);
    for (start, end, after) in [(0, 0, long.as_str()), (3, 1, "x")] {
        let refused = Offer::of(
            RunId::from_bytes([1; 16]),
            &ProposalOffered {
                doc: Address::parse("lab/notes.md").unwrap(),
                baseline: B3Hash::digest(b""),
                start,
                end,
                before: String::new(),
                after: after.to_owned(),
            },
        )
        .unwrap_err();
        assert_eq!(refused.code().as_str(), "E_INVALID_ARGS");
    }
}

/// D17: two cards on one version land as one save; a card made on a
/// version that has moved is refused, while rejecting it writes nothing
/// and is not refused.
#[test]
fn cards_on_the_current_version_land_together_and_a_stale_one_is_refused() {
    let source = "One. Two. Three.";
    let first = offer(1, source, (5, 9, "Deux."));
    let second = offer(1, source, (10, 16, "Trois."));
    let both = decide(
        source.as_bytes(),
        &[
            (&first, &[accept(0), accept(1)]),
            (&second, &[accept(0), accept(1)]),
        ],
    )
    .unwrap()
    .unwrap();
    assert_eq!(both.bytes(), b"One. Deux. Trois.");
    assert_eq!(both.baseline(), B3Hash::digest(source.as_bytes()));
    let stale = decide(both.bytes(), &[(&first, &[accept(0), accept(1)])]).unwrap_err();
    assert_eq!(stale.code().as_str(), "E_VERSION_CONFLICT");
    assert_eq!(decide(both.bytes(), &[(&first, &[])]).unwrap(), None);
}

/// D17: overlapping cards, a card named twice, and a card whose
/// original is not what its version holds there are refused.
#[test]
fn overlapping_twice_named_or_unmatched_cards_are_refused() {
    let source = "One. Two. Three.";
    let wide = offer(1, source, (5, 16, "Deux. Trois."));
    let narrow = offer(1, source, (10, 16, "Trois."));
    let take = [accept(0), accept(1)];
    let overlapping = decide(source.as_bytes(), &[(&narrow, &take), (&wide, &take)]).unwrap_err();
    assert_eq!(overlapping.code().as_str(), "E_INVALID_ARGS");
    let twice = decide(source.as_bytes(), &[(&narrow, &take), (&narrow, &[])]).unwrap_err();
    assert_eq!(twice.code().as_str(), "E_INVALID_ARGS");
    let lying = Offer::of(
        RunId::from_bytes([1; 16]),
        &ProposalOffered {
            doc: Address::parse("lab/notes.md").unwrap(),
            baseline: B3Hash::digest(source.as_bytes()),
            start: 0,
            end: 4,
            before: "Uno.".to_owned(),
            after: "Eins.".to_owned(),
        },
    )
    .unwrap();
    let unmatched = decide(source.as_bytes(), &[(&lying, &[accept(0), accept(1)])]).unwrap_err();
    assert_eq!(unmatched.code().as_str(), "E_INVALID_ARGS");
}
