// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two marks a person reads a row by: where its first painted box
//! starts, and whether a key on it was drawn with a line under it.
//!
//! Both are measurements over the same page the other five properties
//! judge. They live here rather than beside them because both need what
//! the probe reports *about a row's contents* - the first mark's centre
//! and the decoration that reaches a key - which nothing else asks for.
//!
//! **The underline is read as it is drawn at rest.** A decoration a
//! rule paints only while a pointer rests on a row is not in a dumped
//! document, so this measures the page nobody is touching. That is an
//! under-report and never a false one.

use std::collections::{BTreeMap, BTreeSet};

use super::{Drawn, SLACK, violation};
use crate::report::Violation;

/// Every clickable row in a navigation column starts its first mark at
/// the same x.
///
/// The rail's status dot was 8 px wide and its glyphs 18, each centred
/// in the same 12 px of padding, so the dot's centre sat five pixels
/// left of every icon under it. The dot now shares the glyph's box, and
/// this is what holds it there.
///
/// **A column, not a bar.** Rows that all share one top are a row of
/// tabs, and asking them to share an x would ask them to sit on top of
/// one another; a nav whose rows do not stack is left alone.
pub(super) fn rows_share_a_first_mark(drawn: &[Drawn], out: &mut Vec<Violation>) {
    for (at, nav) in drawn
        .iter()
        .enumerate()
        .filter(|(_, held)| held.tag == "NAV" && held.drawn())
    {
        let Ok(index) = i64::try_from(at) else {
            continue;
        };
        let rows: Vec<&Drawn> = drawn
            .iter()
            .filter(|held| {
                held.operable()
                    && held.drawn()
                    && held.first_mark >= 0
                    && descends(drawn, held, index)
            })
            .collect();
        let stacked: BTreeSet<i64> = rows.iter().map(|row| row.top).collect();
        if rows.len() < 2 || stacked.len() < 2 {
            continue;
        }
        let mut marks: BTreeMap<i64, String> = BTreeMap::new();
        for row in &rows {
            marks.entry(row.first_mark).or_insert_with(|| row.called());
        }
        if let (Some(first), Some(last)) = (marks.keys().next(), marks.keys().next_back())
            && last.saturating_sub(*first) <= SLACK
        {
            continue;
        }
        let listed: Vec<String> = marks
            .iter()
            .map(|(centre, name)| format!("{name} at x={centre}"))
            .collect();
        out.push(violation(
            "every clickable row in a navigation column starts its first mark at the same x",
            format!(
                "{} holds {}: {}",
                nav.called(),
                marks.len(),
                listed.join(", ")
            ),
            "draw the first mark of every row in one box of the same size - a dot centred in \
             the glyph's box lines up with the glyphs, a dot centred in its own does not",
        ));
    }
}

/// No key is drawn with a line under it.
///
/// A key is a face, not a link. The stylesheet underlines a link under
/// the pointer, and the mark inside it went with it, so `g c` on the
/// rail read as something to click.
pub(super) fn no_key_is_underlined(drawn: &[Drawn], out: &mut Vec<Violation>) {
    for held in drawn
        .iter()
        .filter(|held| held.tag == "KBD" && held.drawn() && held.underlined)
    {
        out.push(violation(
            "no key is drawn with a line under it",
            format!(
                "{} at x={} y={} carries an underline",
                held.called(),
                held.left,
                held.top
            ),
            "a decoration reaches every in-flow descendant and a `text-decoration: none` on the \
             key does not stop it: keep the key in its own atomic box, or do not decorate what \
             holds it",
        ));
    }
}

/// Whether an element sits under the measured element at `ancestor`.
fn descends(drawn: &[Drawn], held: &Drawn, ancestor: i64) -> bool {
    let mut at = held.parent;
    let mut steps = drawn.len();
    while at >= 0 {
        if at == ancestor {
            return true;
        }
        let Some(next) = usize::try_from(at).ok().and_then(|index| drawn.get(index)) else {
            return false;
        };
        if steps == 0 {
            return false;
        }
        steps = steps.saturating_sub(1);
        at = next.parent;
    }
    false
}
