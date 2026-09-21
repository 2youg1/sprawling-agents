// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One fact, painted in two places (xtask-SPEC.md section 8-26).
//!
//! **This is the repository's own defect, read off the screen.**
//! `AGENTS.md` hunts one family above all others - a fact with more
//! than one home - and every other gate hunts it in the source: a
//! constant beside a literal, two structs describing one thing, a
//! grammar ported by hand across a wire. None of them can see the
//! frontend's version of it, because on a page the second home is not
//! a second declaration. It is a second *box*, and it only exists once
//! the page has been laid out and the queries have answered.
//!
//! The one this instrument was written after is exact. The model in
//! force was drawn twice: once by the composer's chooser and once by
//! the fact strip along the bottom of the window. Two subtrees, two
//! reads of two different answers, no link between them - and the day
//! `/model` in the command palette wrote one of them, the two said
//! different things and the page did not know. No source gate could
//! have found it. Two boxes painting the same string, in two different
//! landmarks, is what it looks like from here.
//!
//! **What is a finding and what is a list.** A table repeats a word
//! down a column and a legend repeats it beside a swatch; those are one
//! fact drawn once and referred to, not two homes. Three conditions
//! separate them, and all three have to hold:
//!
//! 1. the boxes sit in **different landmarks** - a `<nav>` and a
//!    `<main>` are two views of a city, a `<ul>` is one view of a list;
//! 2. there are **few of them** - a string painted six times is a
//!    column, and a column has one writer;
//! 3. the string **carries information** - `—`, `0` and `·` are
//!    punctuation this page uses to mean "nothing here", and they are
//!    supposed to repeat.
//!
//! **It reports; it never refuses.** Two boxes may legitimately say the
//! same word, and the instrument cannot know which. What it can do is
//! put the pair in front of a person with both locations, so that the
//! question - *do these two have one source?* - gets asked once instead
//! of never.

use std::collections::BTreeMap;

use super::{Deviation, Drawn, Finding, Sampled};

/// How short a string has to be before repeating it means nothing.
///
/// Measured in characters rather than bytes: the client is drawn in
/// Chinese as often as in English, and three Han characters are a
/// sentence where three ASCII letters are a word.
const TOO_SHORT: usize = 3;

/// How many times a string may be painted before it is a column rather
/// than a duplicated fact.
///
/// Four is the count at which a person stops reading the instances and
/// starts reading the pattern: a fact with two homes is a defect, a
/// fact with five is a table.
const A_COLUMN: usize = 4;

/// The strings that are punctuation this page uses for absence. They
/// are supposed to repeat, and the day one of them stops repeating is
/// the day something is wrong the other way.
const PUNCTUATION: [&str; 6] = ["—", "–", "…", "·", "n/a", "N/A"];

/// The roles that separate one view of the page from another.
///
/// A landmark is the unit a person navigates between, so two boxes in
/// two landmarks are two places a reader will meet the same fact -
/// which is exactly the case where nobody notices the two disagreeing.
const LANDMARKS: [&str; 7] = [
    "navigation",
    "main",
    "contentinfo",
    "banner",
    "complementary",
    "dialog",
    "form",
];

pub(super) fn one_fact_has_one_home<'a>(drawn: &'a [Drawn], out: &mut Vec<Deviation<'a>>) {
    let mut painted: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (seat, box_) in drawn.iter().enumerate() {
        if !carries_a_fact(box_) {
            continue;
        }
        painted.entry(box_.name.trim()).or_default().push(seat);
    }
    for (text, seats) in painted {
        if seats.len() < 2 || seats.len() > A_COLUMN {
            continue;
        }
        let homes: Vec<(usize, &str)> = seats
            .iter()
            .filter_map(|seat| drawn.get(*seat).map(|_| (*seat, landmark(drawn, *seat))))
            .collect();
        // Every pair in one landmark is one list; a pair across two is
        // two views of one fact.
        let Some((first, where_first)) = homes.first().copied() else {
            continue;
        };
        if homes.iter().all(|(_, held)| *held == where_first) {
            continue;
        }
        for (seat, _) in homes.iter().skip(1) {
            let (Some(at), Some(other)) = (drawn.get(*seat), drawn.get(first)) else {
                continue;
            };
            out.push(Deviation {
                at,
                finding: Finding::Echoed {
                    other,
                    text,
                    homes: homes.len(),
                },
            });
        }
    }
}

/// Whether this box paints something worth comparing: words a person
/// reads, long enough to mean something, and not the punctuation this
/// page uses for absence.
fn carries_a_fact(box_: &Drawn) -> bool {
    if !matches!(box_.sampled, Sampled::Words) || box_.text.is_none() {
        return false;
    }
    let said = box_.name.trim();
    said.chars().count() > TOO_SHORT
        && !PUNCTUATION.contains(&said)
        && said.chars().any(char::is_alphanumeric)
}

/// The landmark a box sits in, walking the measured ancestors until one
/// of them is a region a person navigates to. A box under none of them
/// is in the page itself, which is one place like any other.
fn landmark(drawn: &[Drawn], seat: usize) -> &str {
    let mut here = seat;
    // The tree is finite and `parent` always points nearer the root, so
    // the walk ends; the bound is belt and braces against a probe that
    // ever reports a cycle.
    for _ in 0..drawn.len() {
        let Some(box_) = drawn.get(here) else {
            return "the page";
        };
        if LANDMARKS.contains(&box_.role.as_str()) {
            return box_.role.as_str();
        }
        let Ok(up) = usize::try_from(box_.parent) else {
            return "the page";
        };
        here = up;
    }
    "the page"
}
